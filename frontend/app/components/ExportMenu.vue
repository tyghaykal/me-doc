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
      :aria-label="downloading ? 'Exporting…' : 'Export'"
      class="rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
      @click="open = !open"
    >
      <span class="sm:hidden">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
          <path d="M12 3v12m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
        </svg>
      </span>
      <span class="hidden sm:inline">{{ downloading ? 'Exporting…' : 'Export' }}</span>
    </button>

    <template v-if="open">
      <div class="fixed inset-0 z-40" @click="open = false" />
      <div
        role="menu"
        class="absolute right-0 z-50 mt-1 w-44 rounded-md border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
      >
        <button
          v-for="f in formats"
          :key="f.ext"
          type="button"
          role="menuitem"
          class="block w-full px-3 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="download(f.ext)"
        >
          {{ f.label }}
        </button>
      </div>
    </template>

    <p v-if="error" class="absolute right-0 mt-1 text-xs text-red-600 dark:text-red-400">{{ error }}</p>
  </div>
</template>
