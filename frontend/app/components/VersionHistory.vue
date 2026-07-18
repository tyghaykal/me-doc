<script setup lang="ts">
const props = defineProps<{
  pageId: string
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
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

async function load() {
  loading.value = true
  error.value = null
  try {
    versions.value = await api<Version[]>(`/pages/${props.pageId}/versions`)
  } catch (err: any) {
    error.value = errText(err, 'Failed to load version history.')
  } finally {
    loading.value = false
  }
}

async function restore(id: string) {
  if (!window.confirm('Restore this version? Current content will be replaced.')) return
  restoringId.value = id
  error.value = null
  try {
    await api(`/pages/${props.pageId}/versions/${id}/restore`, { method: 'POST' })
    close()
  } catch (err: any) {
    error.value = errText(err, 'Failed to restore version.')
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
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 font-sans dark:bg-black/60"
      @click.self="close"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Version history"
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-slate-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Version history</h2>
          <button
            type="button"
            aria-label="Close"
            class="text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <p v-if="loading" class="mt-5 text-sm text-slate-500 dark:text-slate-400">Loading…</p>
        <p v-else-if="error" class="mt-5 text-sm text-red-600 dark:text-red-400">{{ error }}</p>
        <p v-else-if="versions.length === 0" class="mt-5 text-sm text-slate-500 dark:text-slate-400">
          No saved versions yet.
        </p>

        <ul v-else class="mt-5 divide-y divide-slate-200 dark:divide-slate-800">
          <li
            v-for="v in versions"
            :key="v.id"
            class="flex items-center justify-between py-3"
          >
            <span class="text-sm text-slate-700 dark:text-slate-300">{{ relativeTime(v.created_at) }}</span>
            <button
              type="button"
              :disabled="restoringId === v.id"
              class="rounded border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
              @click="restore(v.id)"
            >
              {{ restoringId === v.id ? '…' : 'Restore' }}
            </button>
          </li>
        </ul>
      </div>
    </div>
  </Teleport>
</template>
