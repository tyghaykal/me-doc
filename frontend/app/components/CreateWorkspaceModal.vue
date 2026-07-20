<script setup lang="ts">
const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const workspacesStore = useWorkspacesStore()

const name = ref('')
const error = ref<string | null>(null)
const saving = ref(false)

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

async function submit() {
  error.value = null
  saving.value = true
  try {
    const ws = await workspacesStore.create(name.value.trim())
    workspacesStore.setActive(ws)
    name.value = ''
    close()
  } catch (err: any) {
    error.value = errText(err, 'Failed to create workspace.')
  } finally {
    saving.value = false
  }
}

function close() {
  emit('update:open', false)
}
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
        aria-label="Create workspace"
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-slate-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">New workspace</h2>
          <button
            type="button"
            aria-label="Close"
            class="text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <form class="mt-5 flex gap-2" @submit.prevent="submit">
          <input
            v-model="name"
            type="text"
            required
            placeholder="Workspace name"
            class="flex-1 rounded border border-slate-300 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
          />
          <button
            type="submit"
            :disabled="saving"
            class="rounded bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
          >
            {{ saving ? '…' : 'Create' }}
          </button>
        </form>
        <p v-if="error" class="mt-2 text-sm text-red-600 dark:text-red-400">{{ error }}</p>
      </div>
    </div>
  </Teleport>
</template>
