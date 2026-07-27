<script setup lang="ts">
const props = defineProps<{
  pageId: string
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  restored: []
}>()

const api = useApi()

interface Version {
  id: string
  size: number
  created_at: string
}

const versions = ref<Version[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const restoringId = ref<string | null>(null)
const confirmTarget = ref<Version | null>(null)
const successToast = ref(false)
let toastTimer: ReturnType<typeof setTimeout> | undefined

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

function relativeTime(iso: string): string {
  const secs = Math.round((Date.now() - new Date(iso).getTime()) / 1000)
  if (secs < 60) return 'just now'
  const mins = Math.round(secs / 60)
  if (mins < 60) return `${mins} minute${mins === 1 ? '' : 's'} ago`
  const hours = Math.round(mins / 60)
  if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`
  const days = Math.round(hours / 24)
  return `${days} day${days === 1 ? '' : 's'} ago`
}

function absoluteTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      dateStyle: 'medium',
      timeStyle: 'short',
    })
  } catch {
    return iso
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function load() {
  loading.value = true
  error.value = null
  confirmTarget.value = null
  try {
    versions.value = await api<Version[]>(`/pages/${props.pageId}/versions`)
  } catch (err: any) {
    error.value = errText(err, 'Failed to load version history.')
  } finally {
    loading.value = false
  }
}

function askRestore(v: Version) {
  confirmTarget.value = v
  error.value = null
}

function cancelConfirm() {
  if (restoringId.value) return
  confirmTarget.value = null
}

function showSuccessToast() {
  successToast.value = true
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => {
    successToast.value = false
  }, 3200)
}

async function confirmRestore() {
  const v = confirmTarget.value
  if (!v) return
  restoringId.value = v.id
  error.value = null
  try {
    await api(`/pages/${props.pageId}/versions/${v.id}/restore`, { method: 'POST' })
    confirmTarget.value = null
    emit('restored')
    emit('update:open', false)
    showSuccessToast()
  } catch (err: any) {
    error.value = errText(err, 'Failed to restore version.')
  } finally {
    restoringId.value = null
  }
}

function close() {
  if (restoringId.value) return
  confirmTarget.value = null
  emit('update:open', false)
}

onBeforeUnmount(() => {
  clearTimeout(toastTimer)
})

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) load()
  },
)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 font-sans dark:bg-black/60"
      @click.self="close"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Version history"
        class="flex max-h-[min(36rem,85vh)] w-full max-w-md flex-col overflow-hidden rounded-lg bg-white shadow-xl dark:bg-neutral-900"
        @keydown.esc="close"
      >
        <div class="flex shrink-0 items-start justify-between border-b border-neutral-200 px-5 py-4 dark:border-neutral-800">
          <div>
            <h2 class="text-xl font-bold text-neutral-900 dark:text-neutral-100">Version history</h2>
            <p class="mt-0.5 text-xs text-neutral-500 dark:text-neutral-400">
              Snapshots from past editing sessions
            </p>
          </div>
          <button
            type="button"
            aria-label="Close"
            class="rounded p-1 text-neutral-400 hover:bg-neutral-100 hover:text-neutral-600 dark:text-neutral-500 dark:hover:bg-neutral-800 dark:hover:text-neutral-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <div class="thin-scrollbar min-h-0 flex-1 overflow-y-auto px-5 py-3">
          <p v-if="loading" class="py-6 text-center text-sm text-neutral-500 dark:text-neutral-400">
            Loading…
          </p>
          <p v-else-if="error && !confirmTarget" class="py-4 text-sm text-red-600 dark:text-red-400">
            {{ error }}
          </p>
          <p
            v-else-if="versions.length === 0"
            class="py-6 text-center text-sm text-neutral-500 dark:text-neutral-400"
          >
            No saved versions yet. Versions are created when everyone leaves a page.
          </p>

          <ul v-else class="divide-y divide-neutral-200 dark:divide-neutral-800">
            <li
              v-for="v in versions"
              :key="v.id"
              class="flex items-center justify-between gap-3 py-3"
            >
              <div class="min-w-0">
                <p class="truncate text-sm font-medium text-neutral-800 dark:text-neutral-200">
                  {{ relativeTime(v.created_at) }}
                </p>
                <p class="truncate text-xs text-neutral-400 dark:text-neutral-500">
                  {{ absoluteTime(v.created_at) }} · {{ formatSize(v.size) }}
                </p>
              </div>
              <button
                type="button"
                class="shrink-0 rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
                @click="askRestore(v)"
              >
                Restore
              </button>
            </li>
          </ul>
        </div>
      </div>

      <!-- Nested confirm dialog (replaces window.confirm) -->
      <div
        v-if="confirmTarget"
        class="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 p-4"
        @click.self="cancelConfirm"
      >
        <div
          role="alertdialog"
          aria-modal="true"
          aria-labelledby="restore-title"
          aria-describedby="restore-desc"
          class="w-full max-w-sm rounded-lg bg-white p-5 shadow-xl dark:bg-neutral-900"
        >
          <h3 id="restore-title" class="text-base font-semibold text-neutral-900 dark:text-neutral-100">
            Restore this version?
          </h3>
          <p id="restore-desc" class="mt-2 text-sm text-neutral-600 dark:text-neutral-400">
            The page will be replaced with the snapshot from
            <span class="font-medium text-neutral-800 dark:text-neutral-200">{{
              absoluteTime(confirmTarget.created_at)
            }}</span>
            ({{ relativeTime(confirmTarget.created_at) }}). Your current content will be overwritten.
          </p>
          <p v-if="error" class="mt-2 text-sm text-red-600 dark:text-red-400">{{ error }}</p>
          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              :disabled="!!restoringId"
              class="rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="cancelConfirm"
            >
              Cancel
            </button>
            <button
              type="button"
              :disabled="!!restoringId"
              class="rounded bg-teal-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
              @click="confirmRestore"
            >
              {{ restoringId ? 'Restoring…' : 'Restore' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Lives outside the dialog so it stays visible after the popup closes. -->
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="translate-y-2 opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="translate-y-2 opacity-0"
    >
      <div
        v-if="successToast"
        role="status"
        class="fixed bottom-6 left-1/2 z-[70] flex -translate-x-1/2 items-center gap-2 rounded-lg border border-neutral-200 bg-white px-4 py-2.5 text-sm font-medium text-neutral-800 shadow-lg dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
      >
        <span
          class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-100 text-emerald-700 dark:bg-emerald-900/50 dark:text-emerald-300"
          aria-hidden="true"
        >
          ✓
        </span>
        Version restored successfully
      </div>
    </Transition>
  </Teleport>
</template>
