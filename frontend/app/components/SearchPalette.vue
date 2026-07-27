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
const error = ref<string | null>(null)
const inputRef = ref<HTMLInputElement | null>(null)
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
    error.value = null
    return
  }
  loading.value = true
  error.value = null
  try {
    results.value = await pagesStore.searchPages(props.workspaceId, q)
  } catch (e: any) {
    results.value = []
    error.value = e?.data?.message ?? e?.message ?? 'Search failed.'
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
  async (isOpen) => {
    if (!isOpen) {
      query.value = ''
      results.value = []
      error.value = null
      return
    }
    await nextTick()
    inputRef.value?.focus()
    // Teleport + v-if: one more frame after paint
    requestAnimationFrame(() => inputRef.value?.focus())
  },
)
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 p-4 pt-24 font-sans dark:bg-black/60"
      @click.self="close"
    >
      <Transition
        appear
        enter-active-class="transition duration-200 ease-out"
        enter-from-class="opacity-0 scale-95 translate-y-2"
        enter-to-class="opacity-100 scale-100 translate-y-0"
        leave-active-class="transition duration-150 ease-in"
        leave-from-class="opacity-100 scale-100 translate-y-0"
        leave-to-class="opacity-0 scale-95 translate-y-2"
      >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Search"
        class="w-full max-w-lg rounded-lg bg-white shadow-xl dark:bg-neutral-900"
        @keydown.esc="close"
      >
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          placeholder="Search title, content, people…"
          class="w-full border-b border-neutral-200 bg-transparent px-4 py-3 text-sm text-neutral-900 outline-none placeholder:text-neutral-400 dark:border-neutral-800 dark:text-neutral-100"
        />

        <p v-if="loading" class="px-4 py-3 text-sm text-neutral-500 dark:text-neutral-400">Searching…</p>
        <p v-else-if="error" class="px-4 py-3 text-sm text-red-600 dark:text-red-400">{{ error }}</p>
        <p v-else-if="query && results.length === 0" class="px-4 py-3 text-sm text-neutral-500 dark:text-neutral-400">
          No matches.
        </p>

        <ul v-else-if="results.length" class="thin-scrollbar max-h-80 overflow-y-auto py-1">
          <li v-for="page in results" :key="page.id">
            <button
              type="button"
              class="flex w-full cursor-pointer items-center gap-2 truncate px-4 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="select(page)"
            >
              <span class="shrink-0">{{ page.icon || DEFAULT_PAGE_ICON }}</span>
              <span class="truncate">{{ page.title || 'Untitled' }}</span>
            </button>
          </li>
        </ul>
      </div>
      </Transition>
    </div>
    </Transition>
  </Teleport>
</template>
