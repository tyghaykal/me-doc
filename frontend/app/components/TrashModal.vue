<script setup lang="ts">
import { DEFAULT_PAGE_ICON } from '~/stores/pages'

const props = defineProps<{
  workspaceId: string
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const pagesStore = usePagesStore()
const loading = ref(false)
const error = ref<string | null>(null)
const restoringId = ref<string | null>(null)

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

async function load() {
  loading.value = true
  error.value = null
  try {
    await pagesStore.fetchTrash(props.workspaceId)
  } catch (err: any) {
    error.value = errText(err, 'Failed to load trash.')
  } finally {
    loading.value = false
  }
}

async function restore(id: string) {
  restoringId.value = id
  error.value = null
  try {
    await pagesStore.restorePage(id)
  } catch (err: any) {
    error.value = errText(err, 'Failed to restore page.')
  } finally {
    restoringId.value = null
  }
}

function close() {
  emit('update:open', false)
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) load()
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
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 font-sans dark:bg-black/60"
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
        aria-label="Trash"
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-neutral-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-neutral-900 dark:text-neutral-100">Trash</h2>
          <button
            type="button"
            aria-label="Close"
            class="text-neutral-400 hover:text-neutral-600 dark:text-neutral-500 dark:hover:text-neutral-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <p v-if="loading" class="mt-5 text-sm text-neutral-500 dark:text-neutral-400">Loading…</p>
        <p v-else-if="error" class="mt-5 text-sm text-red-600 dark:text-red-400">{{ error }}</p>
        <p v-else-if="pagesStore.trash.length === 0" class="mt-5 text-sm text-neutral-500 dark:text-neutral-400">
          Trash is empty.
        </p>

        <ul v-else class="mt-5 divide-y divide-neutral-200 dark:divide-neutral-800">
          <li
            v-for="page in pagesStore.trash"
            :key="page.id"
            class="flex items-center justify-between py-3"
          >
            <span class="flex min-w-0 items-center gap-1 truncate text-sm text-neutral-700 dark:text-neutral-300">
              <span class="shrink-0">{{ page.icon || DEFAULT_PAGE_ICON }}</span>
              <span class="truncate">{{ page.title || 'Untitled' }}</span>
            </span>
            <button
              type="button"
              :disabled="restoringId === page.id"
              class="rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="restore(page.id)"
            >
              {{ restoringId === page.id ? '…' : 'Restore' }}
            </button>
          </li>
        </ul>
      </div>
      </Transition>
    </div>
    </Transition>
  </Teleport>
</template>
