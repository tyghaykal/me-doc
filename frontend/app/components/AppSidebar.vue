<script setup lang="ts">
defineProps<{ workspaceId: string }>()

const emit = defineEmits<{
  'open-create': []
  'open-members': []
  'open-trash': []
}>()

const pagesStore = usePagesStore()
const { isDark, toggleTheme } = useTheme()

const searchOpen = ref(false)

function goHome() {
  pagesStore.activePageId = null
}

function onKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()
    searchOpen.value = true
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <aside class="thin-scrollbar flex w-64 shrink-0 flex-col gap-3 overflow-y-auto border-r border-neutral-200 bg-neutral-50 p-3 dark:border-neutral-800 dark:bg-neutral-950">
    <div class="rounded border border-neutral-200 dark:border-neutral-800">
      <WorkspaceSwitcher @open-create="emit('open-create')" @open-members="emit('open-members')" />
    </div>

    <div class="flex items-center gap-1">
      <button
        type="button"
        aria-label="Home"
        class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
        @click="goHome"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
          <path d="M3 10.5 12 3l9 7.5" /><path d="M5 9.5V21h14V9.5" />
        </svg>
      </button>
      <button
        type="button"
        aria-label="Search"
        title="Search (Ctrl/Cmd+K)"
        class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
        @click="searchOpen = true"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
          <circle cx="11" cy="11" r="7" /><path d="m21 21-4.3-4.3" />
        </svg>
      </button>
      <button
        type="button"
        aria-label="Toggle theme"
        class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
        @click="toggleTheme"
      >
        <svg v-if="isDark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
          <circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
        </svg>
        <svg v-else xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="h-4 w-4">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </button>
    </div>

    <SidebarRecents />

    <SidebarFavorites />

    <div>
      <p class="mb-1 px-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Private</p>
      <PageTree :nodes="pagesStore.pageTree" :workspace-id="workspaceId" />
    </div>

    <SidebarSharedSection />

    <button
      type="button"
      class="mt-auto flex items-center gap-2 rounded px-2 py-1.5 text-left text-sm text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
      @click="emit('open-trash')"
    >
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4 shrink-0">
        <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6" />
      </svg>
      Trash
    </button>

    <UserMenu />

    <SearchPalette :workspace-id="workspaceId" v-model:open="searchOpen" />
  </aside>
</template>
