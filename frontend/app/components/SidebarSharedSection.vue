<script setup lang="ts">
import { DEFAULT_PAGE_ICON } from '~/stores/pages'

const pagesStore = usePagesStore()

onMounted(() => {
  pagesStore.fetchSharedPages()
})

const sortedShared = computed(() =>
  [...pagesStore.sharedPages].sort(
    (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
  ),
)

function select(id: string) {
  pagesStore.activePageId = id
}
</script>

<template>
  <div v-if="sortedShared.length">
    <p class="mb-1 px-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Shared</p>
    <ul class="text-sm text-neutral-700 dark:text-neutral-300">
      <li v-for="page in sortedShared" :key="page.id">
        <button
          type="button"
          class="flex w-full cursor-pointer items-center gap-1 truncate rounded px-2 py-1 text-left hover:bg-neutral-100 dark:hover:bg-neutral-800"
          :class="pagesStore.activePageId === page.id ? 'bg-neutral-100 font-medium text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100' : ''"
          @click="select(page.id)"
        >
          <span class="shrink-0">{{ page.icon || DEFAULT_PAGE_ICON }}</span>
          <span class="truncate">{{ page.title || 'Untitled' }}</span>
        </button>
      </li>
    </ul>
  </div>
</template>
