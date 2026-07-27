/**
 * Loads the data the authenticated app shell (AppSidebar's page tree, recents,
 * favorites, and workspace switcher) needs, and keeps it in sync when the
 * active workspace changes. Shared by every page that renders AppSidebar.
 *
 * Uses a single `immediate` watch rather than onMounted+watch so it also
 * works on pages without the `auth` middleware, where authStore.workspace
 * may still be null at mount time and only appears later once a background
 * session refresh resolves.
 */
export function useAppShellData() {
  const authStore = useAuthStore()
  const pagesStore = usePagesStore()
  const workspacesStore = useWorkspacesStore()
  const { prune: pruneRecents } = useRecents()

  let switcherLoaded = false
  async function loadWorkspaceSwitcher(workspaceId: string) {
    if (switcherLoaded) return
    switcherLoaded = true
    await workspacesStore.fetchAll()
    const savedId = localStorage.getItem('activeWorkspaceId')
    if (savedId && savedId !== workspaceId) {
      const match = workspacesStore.list.find((ws) => ws.id === savedId)
      if (match) workspacesStore.setActive(match)
    }
  }

  watch(
    () => authStore.workspace?.id,
    async (id, old) => {
      if (!id) return
      if (old) pagesStore.activePageId = null
      await pagesStore.fetchPages(id)
      await Promise.all([pagesStore.fetchSharedPages(), pagesStore.fetchFavoritePages()])
      pruneRecents(pagesStore.knownPageIds())
      await loadWorkspaceSwitcher(id)
    },
    { immediate: true },
  )
}
