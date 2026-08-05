<script setup lang="ts">
import type { Editor } from '@tiptap/vue-3'
import { DEFAULT_PAGE_ICON } from '~/stores/pages'
import type { FileHandle } from '~/composables/useLocalDocs'

definePageMeta({ middleware: ['auth'] })

const authStore = useAuthStore()
const route = useRoute()
const localDocs = useLocalDocs()
// Same data AppSidebar needs on every other page it's mounted on (page tree,
// recents, favorites, workspace switcher) — this page renders AppSidebar too.
useAppShellData()

type PickerOptions = { suggestedName?: string; types: typeof pickerTypes }
const filePickers = () =>
  window as unknown as {
    showOpenFilePicker(o: PickerOptions): Promise<FileHandle[]>
    showSaveFilePicker(o: PickerOptions): Promise<FileHandle>
  }

// Same mechanism `[[pageId]].vue` uses for the collaborative Editor — needed
// here for `.getJSON()`, which HTML alone (the v-model) can't give us.
const editorInstance = shallowRef<Editor | null>(null)
const editorScrollRoot = ref<HTMLElement | null>(null)

const EMPTY = '<p></p>'
const content = ref(EMPTY)

// `title` doubles as the filename on Save As (never auto-renaming an
// already-open file — same "rename is explicit" behavior as most desktop
// editors).
const title = ref('')

const fileName = ref('')
const dirty = ref(false)
// True while a write to disk is in flight — drives the saving indicator.
const saving = ref(false)
// Set once a file has been opened/saved through the File System Access API, so
// plain "Save" writes back to it without re-prompting. Never set on the
// <input type="file"> / <a download> fallback path — those hand back no handle.
const fileHandle = shallowRef<FileHandle | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const error = ref<string | null>(null)

// Firefox and Safari have no File System Access API; they get the classic
// file-input / download-link flow instead, where every save is a "Save As".
const hasFsAccess = import.meta.client && 'showSaveFilePicker' in window

const pickerTypes = [
  { description: 'Markdown document', accept: { 'text/markdown': ['.md', '.markdown'] } },
]

watch(content, () => {
  dirty.value = true
  scheduleAutosave()
})
watch(title, () => {
  dirty.value = true
  scheduleAutosave()
})

// Autosave has two triggers: shortly after the user stops typing (which in
// practice fires right after finishing a line/paragraph — a fresh keystroke
// keeps pushing this back, so it never interrupts mid-word) and, as a
// fallback for a long unbroken typing stretch that keeps rescheduling the
// debounce, a fixed interval regardless of activity. Both are silent no-ops
// without a file handle — writing to disk needs one already established via
// Open or Save As; there's no such thing as a silent *first* save, since
// showSaveFilePicker requires a user gesture.
const AUTOSAVE_DEBOUNCE_MS = 1500
const AUTOSAVE_INTERVAL_MS = 20_000
let autosaveDebounce: ReturnType<typeof setTimeout> | undefined
let autosaveInterval: ReturnType<typeof setInterval> | undefined

function scheduleAutosave() {
  if (!fileHandle.value) return
  clearTimeout(autosaveDebounce)
  autosaveDebounce = setTimeout(() => void autosave(), AUTOSAVE_DEBOUNCE_MS)
}

async function writeToHandle(): Promise<boolean> {
  if (!fileHandle.value) return false
  saving.value = true
  try {
    const writable = await fileHandle.value.createWritable()
    await writable.write(serialize())
    await writable.close()
    dirty.value = false
    return true
  } catch {
    return false
  } finally {
    saving.value = false
  }
}

// Silent by design — no error UI spam on a background tick; the next
// debounce/interval pass (or an explicit manual Save) just retries.
async function autosave() {
  if (!fileHandle.value || !dirty.value) return
  await writeToHandle()
}

onMounted(() => {
  autosaveInterval = setInterval(() => void autosave(), AUTOSAVE_INTERVAL_MS)
})

function confirmDiscard(): boolean {
  return !dirty.value || confirm('Discard unsaved changes to this document?')
}

function titleFromFilename(name: string): string {
  return name.replace(/\.(md|markdown|html?)$/i, '')
}

function setDocument(html: string, name: string, handle: FileHandle | null) {
  content.value = html
  fileName.value = name
  fileHandle.value = handle
  title.value = name ? titleFromFilename(name) : ''
  nextTick(() => {
    dirty.value = false
  })
}

function newDocument() {
  if (!confirmDiscard()) return
  error.value = null
  setDocument(EMPTY, '', null)
}

// The stored file is plain Markdown — same `markdownToHtml` conversion the
// existing server-backed `.md` import already uses (PageTree.vue), so a
// document written here and one imported into a real page behave identically.
// `.html`/`.htm` still opens via the old raw-HTML path (DOMParser never
// executes scripts, and Tiptap's schema drops anything it doesn't recognise,
// so a hand-edited or hostile file can't inject anything) — only so a file
// saved before this became the default doesn't come back as garbled text.
async function loadFile(file: File, handle: FileHandle | null) {
  const text = await file.text()
  const html = /\.html?$/i.test(file.name)
    ? new DOMParser().parseFromString(text, 'text/html').body.innerHTML
    : markdownToHtml(text)
  setDocument(html || EMPTY, file.name, handle)
  // Only a real File System Access handle can be reopened later without a
  // picker — the <input type=file> fallback hands back no handle to record.
  if (handle) void localDocs.record(file.name, handle)
}

async function openDocument() {
  if (!confirmDiscard()) return
  error.value = null
  if (!hasFsAccess) {
    fileInput.value?.click()
    return
  }
  try {
    const [handle] = await filePickers().showOpenFilePicker({ types: pickerTypes })
    if (handle) await loadFile(await handle.getFile(), handle)
  } catch (err) {
    reportUnlessCancelled(err, 'Could not open that file.')
  }
}

// Reopening a document listed in the sidebar's "Local" section — the handle
// itself lives in IndexedDB (see useLocalDocs), reached via the filename
// carried in the query string so this also works after a hard refresh.
async function openFromQuery(name: string) {
  if (!confirmDiscard()) {
    navigateTo('/app/local', { replace: true })
    return
  }
  error.value = null
  const handle = await localDocs.open(name)
  if (!handle) {
    localDocs.remove(name)
    error.value = `Could not reopen "${name}" — it may have moved, or access wasn't re-granted.`
    navigateTo('/app/local', { replace: true })
    return
  }
  await loadFile(await handle.getFile(), handle)
}

watch(
  () => route.query.open,
  (name) => {
    if (typeof name === 'string' && name) void openFromQuery(name)
  },
  { immediate: true },
)

async function onFileInputChange(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) await loadFile(file, null)
  input.value = ''
}

// Same plain-body shape the server-backed export produces (no frontmatter,
// no wrapper) — the document's identity is its filename, exactly like an
// exported page's identity is its slug-derived Content-Disposition filename.
function serialize(): string {
  const json = editorInstance.value?.getJSON()
  return json ? editorJsonToMarkdown(json) : ''
}

// Filesystem-unsafe characters only — unlike the online export's slug (which
// has to be URL-safe), a local file has no such constraint, so the title
// survives a save/reopen round-trip exactly as typed.
function sanitizeFilename(s: string): string {
  return s.trim().replace(/[\\/:*?"<>|]/g, '').replace(/\s+/g, ' ').slice(0, 100)
}

function suggestedName() {
  if (fileName.value) return fileName.value
  return `${sanitizeFilename(title.value) || 'Untitled'}.md`
}

async function saveDocument() {
  if (!fileHandle.value) return saveDocumentAs()
  error.value = null
  if (!(await writeToHandle())) error.value = 'Could not save the file.'
}

async function saveDocumentAs() {
  error.value = null
  const name = suggestedName()

  if (!hasFsAccess) {
    const url = URL.createObjectURL(new Blob([serialize()], { type: 'text/markdown' }))
    const a = document.createElement('a')
    a.href = url
    a.download = name
    a.click()
    URL.revokeObjectURL(url)
    fileName.value = name
    dirty.value = false
    return
  }

  try {
    const handle = await filePickers().showSaveFilePicker({ suggestedName: name, types: pickerTypes })
    const writable = await handle.createWritable()
    await writable.write(serialize())
    await writable.close()
    fileHandle.value = handle
    fileName.value = handle.name
    dirty.value = false
    void localDocs.record(handle.name, handle)
  } catch (err) {
    reportUnlessCancelled(err, 'Could not save the file.')
  }
}

// Dismissing a file picker rejects with AbortError — that's a user choice, not
// a failure worth showing.
function reportUnlessCancelled(err: unknown, message: string) {
  if ((err as DOMException)?.name === 'AbortError') return
  error.value = message
}

function warnOnUnload(e: BeforeUnloadEvent) {
  if (dirty.value) e.preventDefault()
}
onMounted(() => window.addEventListener('beforeunload', warnOnUnload))
onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', warnOnUnload)
  clearTimeout(autosaveDebounce)
  clearInterval(autosaveInterval)
})
</script>

<template>
  <div class="flex h-screen font-sans">
    <AppSidebar
      v-if="authStore.workspace"
      :workspace-id="authStore.workspace.id"
      @open-create="navigateTo('/app')"
      @open-members="navigateTo('/app')"
      @open-trash="navigateTo('/app')"
    />

    <div class="flex min-w-0 flex-1 flex-col bg-white dark:bg-neutral-900">
      <header class="flex items-center justify-between border-b border-neutral-200 px-6 py-3 dark:border-neutral-800">
        <div class="flex min-w-0 items-center gap-2 text-sm">
          <span
            class="flex items-center gap-1 px-1.5 py-1 text-neutral-500 dark:text-neutral-400"
            title="Stored on this device only — never uploaded or shared"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5">
              <rect x="4" y="10" width="16" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" />
            </svg>
            Offline
          </span>
          <span class="shrink-0">{{ DEFAULT_PAGE_ICON }}</span>
          <span class="min-w-0 font-medium text-neutral-900 dark:text-neutral-100">{{ title || 'Untitled' }}</span>
        </div>

        <div class="flex shrink-0 items-center gap-3">
          <span
            class="flex items-center gap-1.5 px-1.5 py-0.5 text-xs text-neutral-400 dark:text-neutral-500"
            :class="saving ? 'text-teal-600 dark:text-teal-400' : ''"
          >
            <svg
              v-if="saving"
              class="h-3 w-3 animate-spin"
              xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            >
              <path d="M21 12a9 9 0 1 1-6.219-8.56" />
            </svg>
            {{ saving ? 'Saving…' : dirty ? 'Unsaved changes' : fileName ? 'Saved' : '' }}
          </span>

          <div class="flex items-center gap-1">
            <button
              type="button"
              aria-label="New document"
              title="New"
              class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
              @click="newDocument"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                <path d="M12 5v14M5 12h14" />
              </svg>
            </button>
            <button
              type="button"
              aria-label="Open document"
              title="Open"
              class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
              @click="openDocument"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z" />
              </svg>
            </button>
          </div>

          <div class="flex items-center gap-2 border-l border-neutral-200 pl-3 dark:border-neutral-700">
            <button
              type="button"
              class="rounded px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="saveDocument"
            >
              Save
            </button>
            <button
              type="button"
              class="rounded bg-teal-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-teal-700 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
              @click="saveDocumentAs"
            >
              Save As
            </button>
          </div>
        </div>
      </header>

      <p v-if="error" class="border-b border-red-200 bg-red-50 px-6 py-2 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300">
        {{ error }}
      </p>

      <input ref="fileInput" type="file" accept=".md,.markdown,text/markdown" class="hidden" @change="onFileInputChange" />

      <main ref="editorScrollRoot" class="min-h-0 min-w-0 flex-1 overflow-y-auto thin-scrollbar p-8">
        <div class="mx-auto flex w-full max-w-6xl items-start justify-center gap-10">
          <div class="min-w-0 w-full max-w-3xl">
            <p class="mb-2 text-4xl leading-none">{{ DEFAULT_PAGE_ICON }}</p>

            <LocalEditor v-model="content" v-model:title="title" @editor-ready="editorInstance = $event" />
          </div>

          <TableOfContents :editor="editorInstance" :scroll-root="editorScrollRoot" />
        </div>
      </main>
    </div>
  </div>
</template>
