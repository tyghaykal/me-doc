<script setup lang="ts">
import type { Page } from '~/stores/pages'

definePageMeta({ middleware: ['auth'] })

const authStore = useAuthStore()
const pagesStore = usePagesStore()
const workspacesStore = useWorkspacesStore()
const { isDark, toggleTheme } = useTheme()
const route = useRoute()

const shareOpen = ref(false)
const historyOpen = ref(false)
const createWorkspaceOpen = ref(false)
const membersOpen = ref(false)
const settingsOpen = ref(false)

// A page shared with this user (via link or a direct grant) that isn't in
// their own workspace's page list.
const sharedPage = ref<Page | null>(null)
const linkToken = computed(() => {
  const q = route.query.link
  return typeof q === 'string' ? q : null
})

const activePage = computed(
  () => pagesStore.pages.find((p) => p.id === pagesStore.activePageId) ?? sharedPage.value,
)

watch(
  () => pagesStore.activePageId,
  async (id) => {
    sharedPage.value = null
    if (!id || pagesStore.pages.some((p) => p.id === id)) return
    try {
      sharedPage.value = await pagesStore.fetchPage(id, linkToken.value)
    } catch {
      sharedPage.value = null
    }
  },
  { immediate: true },
)

// URL is the source of truth for which page is open, so refresh/back/forward
// land on the same document. pagesStore.activePageId mirrors it both ways:
// route -> store keeps direct link/refresh/navigation working, store ->
// route pushes a URL whenever a page is selected elsewhere (e.g. PageTree).
watch(
  () => route.params.pageId,
  (id) => {
    pagesStore.activePageId = (Array.isArray(id) ? id[0] : id) || null
  },
  { immediate: true },
)

watch(
  () => pagesStore.activePageId,
  (id) => {
    const routeId = route.params.pageId
    const currentId = Array.isArray(routeId) ? routeId[0] : routeId
    if ((id || null) !== (currentId || null)) {
      navigateTo(id ? `/app/${id}` : '/app')
    }
  },
)

onMounted(async () => {
  if (!authStore.workspace) return
  await pagesStore.fetchPages(authStore.workspace.id)
  await workspacesStore.fetchAll()
  const savedId = localStorage.getItem('activeWorkspaceId')
  if (savedId && savedId !== authStore.workspace.id) {
    const match = workspacesStore.list.find((ws) => ws.id === savedId)
    if (match) workspacesStore.setActive(match)
  }
})

watch(
  () => authStore.workspace?.id,
  (id, old) => {
    if (id && old && id !== old) {
      pagesStore.activePageId = null
      pagesStore.fetchPages(id)
    }
  },
)

async function logout() {
  await authStore.logout()
  navigateTo('/login')
}
</script>

<template>
  <div v-if="!authStore.workspace" class="min-h-screen bg-white p-8 font-sans text-slate-600 dark:bg-slate-950 dark:text-slate-400">
    No workspace yet.
  </div>

  <div v-else class="flex h-screen font-sans">
    <aside class="w-64 shrink-0 overflow-y-auto border-r border-slate-200 bg-slate-50 p-3 dark:border-slate-800 dark:bg-slate-950">
      <PageTree :nodes="pagesStore.pageTree" :workspace-id="authStore.workspace.id" />
    </aside>

    <div class="flex flex-1 flex-col bg-white dark:bg-slate-900">
      <header class="flex items-center justify-between border-b border-slate-200 px-6 py-3 dark:border-slate-800">
        <WorkspaceSwitcher @open-create="createWorkspaceOpen = true" @open-members="membersOpen = true" />
        <div class="flex items-center gap-2">
          <ExportMenu v-if="activePage" :page-id="activePage.id" />
          <button
            v-if="activePage"
            class="rounded border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
            @click="historyOpen = true"
          >
            History
          </button>
          <button
            v-if="activePage"
            class="rounded border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
            @click="shareOpen = true"
          >
            Share
          </button>
          <button
            type="button"
            aria-label="Toggle theme"
            class="rounded p-1.5 text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
            @click="toggleTheme"
          >
            <svg v-if="isDark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="h-4 w-4">
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
            </svg>
          </button>
          <button
            type="button"
            aria-label="Settings"
            class="rounded p-1.5 text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
            @click="settingsOpen = true"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <button
            class="rounded px-3 py-1.5 text-sm font-medium text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-200"
            @click="logout"
          >
            Log out
          </button>
        </div>
      </header>

      <main class="flex-1 overflow-y-auto p-8">
        <p v-if="!activePage" class="text-slate-500 dark:text-slate-400">Select a page from the sidebar.</p>
        <Editor
          v-else
          :key="activePage.id"
          :page-id="activePage.id"
          :workspace-id="activePage.workspace_id"
          :title="activePage.title"
          :link-token="linkToken"
        />
      </main>
    </div>

    <ShareDialog v-if="activePage" v-model:open="shareOpen" :page-id="activePage.id" />
    <VersionHistory v-if="activePage" v-model:open="historyOpen" :page-id="activePage.id" />
    <CreateWorkspaceModal v-model:open="createWorkspaceOpen" />
    <WorkspaceMembersModal v-model:open="membersOpen" :workspace-id="authStore.workspace.id" />
    <UserSettingsModal v-model:open="settingsOpen" />
  </div>
</template>
