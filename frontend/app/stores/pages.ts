import { defineStore } from 'pinia'

export const DEFAULT_PAGE_ICON = '📄'

export interface Page {
  id: string
  workspace_id: string
  parent_page_id: string | null
  title: string
  slug: string
  order_index: number
  archived_at: string | null
  created_by: string
  created_at: string
  updated_at: string
  icon: string | null
  kind: 'document' | 'diagram'
  role: string | null
  has_children?: boolean | null
}

export interface PageNode extends Page {
  children: PageNode[]
  childrenLoaded?: boolean
  childrenCursor?: string | null
  childrenLoading?: boolean
}

export interface ShareGrant {
  id: string
  principal_type: 'user' | 'link'
  email: string | null
  role: string
  link_token: string | null
  pending: boolean
  created_at: string
}

export interface PageListResponse {
  items: Page[]
  next_cursor: string | null
}

const PAGE_LIMIT = 30

export const usePagesStore = defineStore('pages', () => {
  const api = useApi()

  /** Flat cache of known non-archived pages (roots + any loaded children). */
  const pages = ref<Page[]>([])
  const activePageId = ref<string | null>(null)
  const sharedPages = ref<Page[]>([])
  const favoritePages = ref<Page[]>([])
  const trash = ref<Page[]>([])

  const rootsCursor = ref<string | null>(null)
  const rootsLoading = ref(false)
  const rootsFullyLoaded = ref(false)

  /** Parent id → next_cursor for that parent's children list (null = no more). */
  const childrenCursorByParent = ref<Record<string, string | null>>({})
  /** Parent ids whose children have been requested at least once. */
  const childrenLoadedParents = ref<Set<string>>(new Set())
  const childrenLoadingParents = ref<Set<string>>(new Set())

  const pendingImportHtml = ref<Record<string, string>>({})
  function setPendingImport(pageId: string, html: string) {
    pendingImportHtml.value[pageId] = html
  }
  function takePendingImport(pageId: string): string | null {
    const html = pendingImportHtml.value[pageId]
    if (html !== undefined) delete pendingImportHtml.value[pageId]
    return html ?? null
  }

  function mergePages(incoming: Page[]) {
    const byId = new Map(pages.value.map((p) => [p.id, p]))
    for (const p of incoming) byId.set(p.id, { ...byId.get(p.id), ...p })
    pages.value = Array.from(byId.values())
  }

  function removePageLocal(pageId: string) {
    pages.value = pages.value.filter((p) => p.id !== pageId)
    favoritePages.value = favoritePages.value.filter((p) => p.id !== pageId)
    sharedPages.value = sharedPages.value.filter((p) => p.id !== pageId)
  }

  const pageTree = computed<PageNode[]>(() => {
    const nodes = new Map<string, PageNode>()
    for (const p of pages.value) {
      nodes.set(p.id, {
        ...p,
        children: [],
        childrenLoaded: childrenLoadedParents.value.has(p.id),
        childrenCursor: childrenCursorByParent.value[p.id] ?? null,
        childrenLoading: childrenLoadingParents.value.has(p.id),
      })
    }

    const roots: PageNode[] = []
    for (const node of nodes.values()) {
      const parent = node.parent_page_id ? nodes.get(node.parent_page_id) : null
      if (parent) parent.children.push(node)
      else if (!node.parent_page_id) roots.push(node)
      // Orphan children whose parent isn't loaded yet stay out of roots.
    }

    // order_index first so drag-reorder sticks; updated_at only breaks ties.
    // (Sorting only by updated_at made reorders jump back after refresh.)
    const sortNodes = (list: PageNode[]) => {
      list.sort(
        (a, b) =>
          a.order_index - b.order_index ||
          new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime() ||
          a.id.localeCompare(b.id),
      )
      for (const n of list) sortNodes(n.children)
    }
    sortNodes(roots)
    return roots
  })

  async function fetchRootPages(workspaceId: string, opts: { reset?: boolean } = {}) {
    if (rootsLoading.value) return
    if (!opts.reset && rootsFullyLoaded.value) return

    if (opts.reset) {
      pages.value = []
      rootsCursor.value = null
      rootsFullyLoaded.value = false
      childrenCursorByParent.value = {}
      childrenLoadedParents.value = new Set()
      childrenLoadingParents.value = new Set()
    }

    rootsLoading.value = true
    try {
      const res = await api<PageListResponse>(`/workspaces/${workspaceId}/pages`, {
        query: {
          limit: PAGE_LIMIT,
          ...(rootsCursor.value ? { cursor: rootsCursor.value } : {}),
        },
      })
      mergePages(res.items)
      rootsCursor.value = res.next_cursor
      rootsFullyLoaded.value = !res.next_cursor
    } finally {
      rootsLoading.value = false
    }
  }

  /** Initial load / workspace switch — first page of roots. */
  async function fetchPages(workspaceId: string) {
    await fetchRootPages(workspaceId, { reset: true })
  }

  async function loadMoreRoots(workspaceId: string) {
    if (rootsFullyLoaded.value || rootsLoading.value) return
    await fetchRootPages(workspaceId)
  }

  async function fetchChildPages(parentId: string, opts: { reset?: boolean } = {}) {
    if (childrenLoadingParents.value.has(parentId)) return
    if (!opts.reset && childrenLoadedParents.value.has(parentId) && !childrenCursorByParent.value[parentId]) {
      return
    }

    const parent = pages.value.find((p) => p.id === parentId)
    const workspaceId = parent?.workspace_id
    if (!workspaceId) return

    childrenLoadingParents.value = new Set(childrenLoadingParents.value).add(parentId)
    try {
      const cursor = opts.reset ? undefined : childrenCursorByParent.value[parentId] || undefined
      const res = await api<PageListResponse>(`/workspaces/${workspaceId}/pages`, {
        query: {
          parent_id: parentId,
          limit: PAGE_LIMIT,
          ...(cursor ? { cursor } : {}),
        },
      })
      if (opts.reset) {
        // Drop previous children of this parent from cache, then merge.
        pages.value = pages.value.filter((p) => p.parent_page_id !== parentId)
      }
      mergePages(res.items)
      childrenCursorByParent.value = {
        ...childrenCursorByParent.value,
        [parentId]: res.next_cursor,
      }
      const loaded = new Set(childrenLoadedParents.value)
      loaded.add(parentId)
      childrenLoadedParents.value = loaded
    } finally {
      const loading = new Set(childrenLoadingParents.value)
      loading.delete(parentId)
      childrenLoadingParents.value = loading
    }
  }

  async function fetchSharedPages() {
    sharedPages.value = await api<Page[]>('/me/shared-pages')
  }

  async function fetchFavoritePages() {
    favoritePages.value = await api<Page[]>('/me/favorite-pages')
  }

  async function favoritePage(pageId: string) {
    await api(`/pages/${pageId}/favorite`, { method: 'POST' })
    await fetchFavoritePages()
  }

  async function unfavoritePage(pageId: string) {
    await api(`/pages/${pageId}/favorite`, { method: 'DELETE' })
    favoritePages.value = favoritePages.value.filter((p) => p.id !== pageId)
  }

  async function fetchDiagrams(workspaceId: string) {
    return api<Page[]>(`/workspaces/${workspaceId}/diagrams`)
  }

  async function searchPages(workspaceId: string, q: string) {
    return api<Page[]>(`/workspaces/${workspaceId}/search`, { query: { q } })
  }

  async function fetchTrash(workspaceId: string) {
    trash.value = await api<Page[]>(`/workspaces/${workspaceId}/pages/trash`)
  }

  async function restorePage(pageId: string) {
    const page = await api<Page>(`/pages/${pageId}/restore`, { method: 'PATCH' })
    trash.value = trash.value.filter((p) => p.id !== pageId)
    await fetchPages(page.workspace_id)
    return page
  }

  async function fetchPage(pageId: string, linkToken?: string | null) {
    return api<Page>(`/pages/${pageId}`, { query: linkToken ? { link: linkToken } : undefined })
  }

  async function createPage(
    workspaceId: string,
    opts: { title?: string; parentPageId?: string; kind?: 'document' | 'diagram' } = {},
  ) {
    const page = await api<Page>(`/workspaces/${workspaceId}/pages`, {
      method: 'POST',
      body: { title: opts.title, parent_page_id: opts.parentPageId, kind: opts.kind },
    })
    if (opts.parentPageId) {
      await fetchChildPages(opts.parentPageId, { reset: true })
      // Parent now has children.
      const parent = pages.value.find((p) => p.id === opts.parentPageId)
      if (parent) mergePages([{ ...parent, has_children: true }])
    } else {
      await fetchPages(workspaceId)
    }
    activePageId.value = page.id
    return page
  }

  async function updatePage(
    pageId: string,
    changes: { title?: string; parentPageId?: string | null; orderIndex?: number; icon?: string | null },
  ) {
    const body: Record<string, unknown> = {}
    if (changes.title !== undefined) body.title = changes.title
    if (changes.parentPageId !== undefined) body.parent_page_id = changes.parentPageId
    if (changes.orderIndex !== undefined) body.order_index = changes.orderIndex
    if (changes.icon !== undefined) body.icon = changes.icon

    const page = await api<Page>(`/pages/${pageId}`, { method: 'PATCH', body })
    // Patch local cache immediately so tree reflects parent/order without
    // waiting for a full roots re-fetch (which would drop loaded children).
    const idx = pages.value.findIndex((p) => p.id === pageId)
    if (idx !== -1) {
      const next = [...pages.value]
      next[idx] = {
        ...next[idx],
        ...page,
        parent_page_id:
          changes.parentPageId !== undefined ? changes.parentPageId : next[idx].parent_page_id,
        order_index:
          changes.orderIndex !== undefined ? changes.orderIndex : next[idx].order_index,
        title: changes.title !== undefined ? changes.title : next[idx].title,
        icon: changes.icon !== undefined ? changes.icon : next[idx].icon,
      }
      if (changes.parentPageId) {
        const pIdx = next.findIndex((p) => p.id === changes.parentPageId)
        if (pIdx !== -1) next[pIdx] = { ...next[pIdx], has_children: true }
      }
      pages.value = next
    }
    // Only full-reset roots when we need a clean workspace view (e.g. title-only
    // edits can skip). Parent moves already update local cache above.
    if (page.workspace_id && changes.parentPageId === undefined && changes.orderIndex === undefined) {
      await fetchPages(page.workspace_id)
    }
    return page
  }

  async function deletePage(pageId: string) {
    const workspaceId = pages.value.find((p) => p.id === pageId)?.workspace_id
    await api<{ message: string }>(`/pages/${pageId}`, { method: 'DELETE' })
    removePageLocal(pageId)
    if (activePageId.value === pageId) activePageId.value = null
    if (import.meta.client) {
      try {
        const { remove } = useRecents()
        remove(pageId)
      } catch {
        /* ignore */
      }
    }
    if (workspaceId) await fetchPages(workspaceId)
  }

  async function listShares(pageId: string) {
    return api<ShareGrant[]>(`/pages/${pageId}/permissions`)
  }

  async function revokeShare(permissionId: string) {
    await api(`/permissions/${permissionId}`, { method: 'DELETE' })
  }

  async function updateShareRole(permissionId: string, role: 'viewer' | 'editor') {
    await api(`/permissions/${permissionId}`, { method: 'PATCH', body: { role } })
  }

  async function duplicatePage(pageId: string) {
    const workspaceId = pages.value.find((p) => p.id === pageId)?.workspace_id
    const page = await api<Page>(`/pages/${pageId}/duplicate`, { method: 'POST' })
    if (workspaceId) await fetchPages(workspaceId)
    activePageId.value = page.id
    return page
  }

  /** All known page ids (active cache + shared + favorites) for recents prune. */
  function knownPageIds(): Set<string> {
    const ids = new Set(pages.value.map((p) => p.id))
    for (const p of sharedPages.value) ids.add(p.id)
    for (const p of favoritePages.value) ids.add(p.id)
    return ids
  }

  return {
    pages,
    activePageId,
    sharedPages,
    favoritePages,
    trash,
    pageTree,
    rootsCursor,
    rootsLoading,
    rootsFullyLoaded,
    childrenCursorByParent,
    childrenLoadedParents,
    childrenLoadingParents,
    fetchPages,
    fetchRootPages,
    loadMoreRoots,
    fetchChildPages,
    fetchPage,
    fetchSharedPages,
    fetchFavoritePages,
    favoritePage,
    unfavoritePage,
    searchPages,
    fetchDiagrams,
    fetchTrash,
    restorePage,
    createPage,
    updatePage,
    deletePage,
    duplicatePage,
    listShares,
    revokeShare,
    updateShareRole,
    setPendingImport,
    takePendingImport,
    knownPageIds,
  }
})
