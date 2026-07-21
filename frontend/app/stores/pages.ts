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
  role: string | null
}

export interface PageNode extends Page {
  children: PageNode[]
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

export const usePagesStore = defineStore('pages', () => {
  const api = useApi()

  const pages = ref<Page[]>([])
  const activePageId = ref<string | null>(null)
  const sharedPages = ref<Page[]>([])
  const favoritePages = ref<Page[]>([])
  const trash = ref<Page[]>([])

  // Content from an imported .txt/.md file, staged for the freshly-created
  // page's Editor to apply on mount (Editor.vue owns the Tiptap instance, so
  // this is how a newly created page hands it its starting content).
  const pendingImportHtml = ref<Record<string, string>>({})
  function setPendingImport(pageId: string, html: string) {
    pendingImportHtml.value[pageId] = html
  }
  function takePendingImport(pageId: string): string | null {
    const html = pendingImportHtml.value[pageId]
    if (html !== undefined) delete pendingImportHtml.value[pageId]
    return html ?? null
  }

  const pageTree = computed<PageNode[]>(() => {
    const nodes = new Map<string, PageNode>()
    for (const p of pages.value) nodes.set(p.id, { ...p, children: [] })

    const roots: PageNode[] = []
    for (const node of nodes.values()) {
      const parent = node.parent_page_id ? nodes.get(node.parent_page_id) : null
      if (parent) parent.children.push(node)
      else roots.push(node)
    }
    return roots
  })

  async function fetchPages(workspaceId: string) {
    pages.value = await api<Page[]>(`/workspaces/${workspaceId}/pages`)
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

  // For a page shared with this user that isn't in their own workspace's
  // list above (a link visit, or a page-level grant on a foreign workspace).
  async function fetchPage(pageId: string, linkToken?: string | null) {
    return api<Page>(`/pages/${pageId}`, { query: linkToken ? { link: linkToken } : undefined })
  }

  async function createPage(
    workspaceId: string,
    opts: { title?: string; parentPageId?: string } = {},
  ) {
    const page = await api<Page>(`/workspaces/${workspaceId}/pages`, {
      method: 'POST',
      body: { title: opts.title, parent_page_id: opts.parentPageId },
    })
    await fetchPages(workspaceId)
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
    if (page.workspace_id) await fetchPages(page.workspace_id)
    return page
  }

  async function deletePage(pageId: string) {
    const workspaceId = pages.value.find((p) => p.id === pageId)?.workspace_id
    await api<{ message: string }>(`/pages/${pageId}`, { method: 'DELETE' })
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

  return {
    pages,
    activePageId,
    sharedPages,
    favoritePages,
    trash,
    pageTree,
    fetchPages,
    fetchPage,
    fetchSharedPages,
    fetchFavoritePages,
    favoritePage,
    unfavoritePage,
    searchPages,
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
  }
})
