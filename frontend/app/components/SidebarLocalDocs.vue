<script setup lang="ts">
import { DEFAULT_PAGE_ICON } from '~/stores/pages'

const route = useRoute()
const { docs } = useLocalDocs()

const activeName = computed(() => (route.path === '/app/local' ? route.query.open : null))

function select(name: string) {
  navigateTo(`/app/local?open=${encodeURIComponent(name)}`)
}
</script>

<template>
  <div v-if="docs.length">
    <p class="mb-1 px-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Local</p>
    <ul class="text-sm text-neutral-700 dark:text-neutral-300">
      <li v-for="entry in docs" :key="entry.name">
        <button
          type="button"
          class="flex w-full items-center gap-1 truncate rounded px-2 py-1 text-left"
          :class="activeName === entry.name
            ? 'bg-teal-50 font-medium text-teal-900 hover:bg-teal-100 dark:bg-teal-950/40 dark:text-teal-200 dark:hover:bg-teal-950/60'
            : 'hover:bg-neutral-100 dark:hover:bg-neutral-800'"
          @click="select(entry.name)"
        >
          <span class="shrink-0">{{ DEFAULT_PAGE_ICON }}</span>
          <span class="truncate">{{ entry.name }}</span>
        </button>
      </li>
    </ul>
  </div>
</template>
