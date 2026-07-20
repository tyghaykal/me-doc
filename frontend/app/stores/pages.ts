import { defineStore } from 'pinia'

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
}

export interface PageNode extends Page {
  children: PageNode[]
}

export const usePagesStore = defineStore('pages', () => {
  const api = useApi()

  const pages = ref<Page[]>([])
  const activePageId = ref<string | null>(null)

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
    changes: { title?: string; parentPageId?: string | null; orderIndex?: number },
  ) {
    const body: Record<string, unknown> = {}
    if (changes.title !== undefined) body.title = changes.title
    if (changes.parentPageId !== undefined) body.parent_page_id = changes.parentPageId
    if (changes.orderIndex !== undefined) body.order_index = changes.orderIndex

    const page = await api<Page>(`/pages/${pageId}`, { method: 'PATCH', body })
    if (page.workspace_id) await fetchPages(page.workspace_id)
    return page
  }

  async function deletePage(pageId: string) {
    const workspaceId = pages.value.find((p) => p.id === pageId)?.workspace_id
    await api<{ message: string }>(`/pages/${pageId}`, { method: 'DELETE' })
    if (workspaceId) await fetchPages(workspaceId)
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
    pageTree,
    fetchPages,
    fetchPage,
    createPage,
    updatePage,
    deletePage,
    duplicatePage,
  }
})
