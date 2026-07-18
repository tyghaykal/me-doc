<script setup lang="ts">
const props = defineProps<{
  pageId: string
}>()

const api = useApi()

const open = ref(false)
const downloading = ref<string | null>(null)
const error = ref<string | null>(null)

const formats = [
  { ext: 'md', label: 'Markdown (.md)' },
  { ext: 'docx', label: 'Word (.docx)' },
  { ext: 'pdf', label: 'PDF (.pdf)' },
]

async function download(ext: string) {
  open.value = false
  error.value = null
  downloading.value = ext
  try {
    // Content-Disposition isn't CORS-exposed (backend only exposes default headers),
    // so name the file client-side from the page id.
    const blob = await api<Blob>(`/pages/${props.pageId}/export?format=${ext}`, {
      responseType: 'blob',
    })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${props.pageId}.${ext}`
    a.click()
    URL.revokeObjectURL(url)
  } catch (err: any) {
    error.value = err?.data?.message ?? err?.message ?? 'Export failed.'
  } finally {
    downloading.value = null
  }
}
</script>

<template>
  <div class="relative">
    <button
      type="button"
      class="rounded border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
      @click="open = !open"
    >
      {{ downloading ? 'Exporting…' : 'Export' }}
    </button>

    <template v-if="open">
      <div class="fixed inset-0 z-40" @click="open = false" />
      <div
        role="menu"
        class="absolute right-0 z-50 mt-1 w-44 rounded-md border border-slate-200 bg-white py-1 shadow-lg dark:border-slate-700 dark:bg-slate-900"
      >
        <button
          v-for="f in formats"
          :key="f.ext"
          type="button"
          role="menuitem"
          class="block w-full px-3 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-slate-800"
          @click="download(f.ext)"
        >
          {{ f.label }}
        </button>
      </div>
    </template>

    <p v-if="error" class="absolute right-0 mt-1 text-xs text-red-600 dark:text-red-400">{{ error }}</p>
  </div>
</template>
