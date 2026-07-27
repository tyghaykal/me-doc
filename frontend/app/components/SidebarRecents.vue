<script setup lang="ts">
import { DEFAULT_PAGE_ICON } from '~/stores/pages'

const pagesStore = usePagesStore()
const { recents, remove } = useRecents()

async function select(id: string) {
  // Verify the page still exists / isn't archived before opening.
  try {
    await pagesStore.fetchPage(id)
    pagesStore.activePageId = id
  } catch {
    remove(id)
  }
}
</script>

<template>
  <div v-if="recents.length">
    <p class="mb-1 px-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Recents</p>
    <ul class="text-sm text-neutral-700 dark:text-neutral-300">
      <li v-for="entry in recents" :key="entry.id">
        <button
          type="button"
          class="flex w-full items-center gap-1 truncate rounded px-2 py-1 text-left"
          :class="pagesStore.activePageId === entry.id
            ? 'bg-teal-50 font-medium text-teal-900 hover:bg-teal-100 dark:bg-teal-950/40 dark:text-teal-200 dark:hover:bg-teal-950/60'
            : 'hover:bg-neutral-100 dark:hover:bg-neutral-800'"
          @click="select(entry.id)"
        >
          <span class="shrink-0">{{ entry.icon || DEFAULT_PAGE_ICON }}</span>
          <span class="truncate">{{ entry.title || 'Untitled' }}</span>
        </button>
      </li>
    </ul>
  </div>
</template>
