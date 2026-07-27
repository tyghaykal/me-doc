<script setup lang="ts">
import type { Page } from '~/stores/pages'
import { DEFAULT_PAGE_ICON } from '~/stores/pages'

const props = defineProps<{ workspaceId: string }>()
const emit = defineEmits<{ pick: [Page]; close: [] }>()

const pagesStore = usePagesStore()
const diagrams = ref<Page[]>([])
const loading = ref(true)
const query = ref('')
const searchInput = ref<HTMLInputElement | null>(null)

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return diagrams.value
  return diagrams.value.filter((d) => (d.title || 'Untitled').toLowerCase().includes(q))
})

onMounted(async () => {
  // Focus the search on open (autofocus is unreliable for freshly-mounted nodes).
  await nextTick()
  searchInput.value?.focus()
  try {
    diagrams.value = await pagesStore.fetchDiagrams(props.workspaceId)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="fixed inset-0 z-[70] flex items-start justify-center bg-black/40 p-4 pt-[12vh]" @click.self="emit('close')">
    <div class="w-full max-w-md overflow-hidden rounded-xl border border-neutral-200 bg-white shadow-2xl dark:border-neutral-700 dark:bg-neutral-900">
      <div class="border-b border-neutral-200 p-2 dark:border-neutral-800">
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          placeholder="Search diagrams…"
          class="w-full rounded-lg bg-neutral-100 px-3 py-2 text-sm text-neutral-800 outline-none placeholder:text-neutral-400 dark:bg-neutral-800 dark:text-neutral-100"
        />
      </div>

      <div class="max-h-72 overflow-y-auto p-1">
        <p v-if="loading" class="px-3 py-6 text-center text-sm text-neutral-400">Loading…</p>
        <p v-else-if="!filtered.length" class="px-3 py-6 text-center text-sm text-neutral-400">
          {{ diagrams.length ? 'No matches.' : 'No diagrams yet — create one from the sidebar.' }}
        </p>
        <button
          v-for="d in filtered"
          :key="d.id"
          type="button"
          class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left hover:bg-neutral-100 dark:hover:bg-neutral-800"
          @click="emit('pick', d)"
        >
          <span>{{ d.icon || DEFAULT_PAGE_ICON }}</span>
          <span class="truncate text-sm text-neutral-700 dark:text-neutral-200">{{ d.title || 'Untitled' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
