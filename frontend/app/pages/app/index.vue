<script setup lang="ts">
definePageMeta({ middleware: ['auth'] })

const authStore = useAuthStore()
const pagesStore = usePagesStore()
const { isDark, toggleTheme } = useTheme()

const shareOpen = ref(false)
const historyOpen = ref(false)

const activePage = computed(() =>
  pagesStore.pages.find((p) => p.id === pagesStore.activePageId) ?? null,
)

onMounted(() => {
  if (authStore.workspace) pagesStore.fetchPages(authStore.workspace.id)
})

async function logout() {
  await authStore.logout()
  navigateTo('/login')
}
</script>

<template>
  <div v-if="!authStore.workspace" class="p-8 font-sans text-slate-600 dark:text-slate-400">
    No workspace yet.
  </div>

  <div v-else class="flex h-screen font-sans">
    <aside class="w-64 shrink-0 overflow-y-auto border-r border-slate-200 bg-slate-50 p-3 dark:border-slate-800 dark:bg-slate-950">
      <PageTree :nodes="pagesStore.pageTree" />
    </aside>

    <div class="flex flex-1 flex-col">
      <header class="flex items-center justify-between border-b border-slate-200 px-6 py-3 dark:border-slate-800">
        <h1 class="text-lg font-semibold text-slate-900 dark:text-slate-100">{{ authStore.workspace.name }}</h1>
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
          :workspace-id="authStore.workspace.id"
        />
      </main>
    </div>

    <ShareDialog v-if="activePage" v-model:open="shareOpen" :page-id="activePage.id" />
    <VersionHistory v-if="activePage" v-model:open="historyOpen" :page-id="activePage.id" />
  </div>
</template>
