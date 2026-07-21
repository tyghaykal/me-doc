<script setup lang="ts">
import { DEFAULT_PAGE_ICON, type Page } from '~/stores/pages'

const props = defineProps<{
  workspaceId: string
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const pagesStore = usePagesStore()
const query = ref('')
const results = ref<Page[]>([])
const loading = ref(false)
let debounceId: ReturnType<typeof setTimeout> | undefined

function close() {
  emit('update:open', false)
}

function select(page: Page) {
  pagesStore.activePageId = page.id
  close()
}

async function runSearch(q: string) {
  if (!q.trim()) {
    results.value = []
    return
  }
  loading.value = true
  try {
    results.value = await pagesStore.searchPages(props.workspaceId, q)
  } finally {
    loading.value = false
  }
}

watch(query, (q) => {
  clearTimeout(debounceId)
  debounceId = setTimeout(() => runSearch(q), 250)
})

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) {
      query.value = ''
      results.value = []
    }
  },
)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 p-4 pt-24 font-sans dark:bg-black/60"
      @click.self="close"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Search"
        class="w-full max-w-lg rounded-lg bg-white shadow-xl dark:bg-neutral-900"
        @keydown.esc="close"
      >
        <input
          v-model="query"
          type="text"
          autofocus
          placeholder="Search pages…"
          class="w-full border-b border-neutral-200 bg-transparent px-4 py-3 text-sm text-neutral-900 outline-none placeholder:text-neutral-400 dark:border-neutral-800 dark:text-neutral-100"
        />

        <p v-if="loading" class="px-4 py-3 text-sm text-neutral-500 dark:text-neutral-400">Searching…</p>
        <p v-else-if="query && results.length === 0" class="px-4 py-3 text-sm text-neutral-500 dark:text-neutral-400">
          No matches.
        </p>

        <ul v-else-if="results.length" class="max-h-80 overflow-y-auto py-1">
          <li v-for="page in results" :key="page.id">
            <button
              type="button"
              class="flex w-full items-center gap-2 truncate px-4 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="select(page)"
            >
              <span class="shrink-0">{{ page.icon || DEFAULT_PAGE_ICON }}</span>
              <span class="truncate">{{ page.title || 'Untitled' }}</span>
            </button>
          </li>
        </ul>
      </div>
    </div>
  </Teleport>
</template>
