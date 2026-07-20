<script setup lang="ts">
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import { BubbleMenu } from '@tiptap/vue-3/menus'
import { DragHandle } from '@tiptap/extension-drag-handle-vue-3'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import { TextStyle } from '@tiptap/extension-text-style'
import Color from '@tiptap/extension-color'
import Highlight from '@tiptap/extension-highlight'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import Collaboration from '@tiptap/extension-collaboration'
import CollaborationCaret from '@tiptap/extension-collaboration-caret'
import { SlashCommand } from './slash-command'

const props = defineProps<{
  pageId: string
  workspaceId: string
  title: string
  linkToken?: string | null
}>()

const api = useApi()
const auth = useAuthStore()
const pagesStore = usePagesStore()
const config = useRuntimeConfig()
const minioBase = config.public.minioBase

const doc = new Y.Doc()

// --- Title ---
const titleDraft = ref(props.title)
watch(
  () => props.title,
  (t) => {
    titleDraft.value = t
  },
)
let titleTimer: ReturnType<typeof setTimeout> | undefined
function scheduleTitleSave() {
  clearTimeout(titleTimer)
  titleTimer = setTimeout(() => {
    pagesStore.updatePage(props.pageId, { title: titleDraft.value || 'Untitled' })
  }, 800)
}

// Stable-ish HSL color from the user id, so a user keeps the same cursor color.
function userColor(id: string): string {
  let hash = 0
  for (let i = 0; i < id.length; i++) hash = (hash * 31 + id.charCodeAt(i)) | 0
  return `hsl(${Math.abs(hash) % 360}, 70%, 50%)`
}

let saveTimer: ReturnType<typeof setTimeout> | undefined

function scheduleSave() {
  clearTimeout(saveTimer)
  saveTimer = setTimeout(save, 1500)
}

async function save() {
  const update = Y.encodeStateAsUpdate(doc)
  await api(`/pages/${props.pageId}/content`, {
    method: 'PUT',
    body: update,
    headers: { 'Content-Type': 'application/octet-stream' },
    query: props.linkToken ? { link: props.linkToken } : undefined,
  })
}

async function uploadImage(file: File): Promise<string> {
  const { upload_url, s3_key } = await api<{ upload_url: string; s3_key: string }>(
    '/attachments/presign',
    {
      method: 'POST',
      body: { workspace_id: props.workspaceId, filename: file.name, content_type: file.type },
    },
  )
  await fetch(upload_url, { method: 'PUT', body: file, headers: { 'Content-Type': file.type } })
  return `${minioBase}/${s3_key}`
}

function imagesFrom(items: DataTransferItemList | FileList | undefined): File[] {
  if (!items) return []
  const out: File[] = []
  for (const it of Array.from(items as ArrayLike<DataTransferItem | File>)) {
    const file = it instanceof File ? it : it.getAsFile()
    if (file && file.type.startsWith('image/')) out.push(file)
  }
  return out
}

async function insertImages(files: File[]) {
  for (const file of files) {
    const src = await uploadImage(file)
    editor.value?.chain().focus().setImage({ src }).run()
  }
}

// --- Slash-command image insertion (opens a hidden file input) ---
const imageInput = ref<HTMLInputElement | null>(null)
function triggerImagePicker() {
  imageInput.value?.click()
}
function onImageInputChange(e: Event) {
  const files = Array.from((e.target as HTMLInputElement).files ?? [])
  if (files.length) insertImages(files)
  ;(e.target as HTMLInputElement).value = ''
}

// --- Drag handle / block menu ---
const hoveredBlock = ref<{ node: any; pos: number } | null>(null)
const blockMenu = ref<{ x: number; y: number; node: any; pos: number } | null>(null)

function onDragHandleNodeChange(data: { node: any; pos: number }) {
  hoveredBlock.value = data.node ? { node: data.node, pos: data.pos } : null
}

function openBlockMenu(e: MouseEvent) {
  if (!hoveredBlock.value || !editor.value) return
  const { node, pos } = hoveredBlock.value
  editor.value.chain().focus().setNodeSelection(pos).run()
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  blockMenu.value = { x: rect.right + 4, y: rect.top, node, pos }
}

const textColors = [
  { name: 'Default', value: null },
  { name: 'Gray', value: '#787774' },
  { name: 'Brown', value: '#9F6B53' },
  { name: 'Orange', value: '#D9730D' },
  { name: 'Green', value: '#448361' },
  { name: 'Blue', value: '#337EA9' },
  { name: 'Purple', value: '#9065B0' },
  { name: 'Red', value: '#D44C47' },
]

function setSelectionColor(value: string | null) {
  const chain = editor.value?.chain().focus()
  if (!chain) return
  if (value) chain.setColor(value).run()
  else chain.unsetColor().run()
}

// useEditor() registers Vue lifecycle hooks (onMounted/onBeforeUnmount)
// internally, so it must be called directly in setup — not inside our own
// onMounted (a running onMounted callback has no active component instance
// either, so nested hook registration silently breaks the same way an
// await would) and its return value (already a ShallowRef<Editor>) must be
// used as-is, not wrapped in a second ref, or EditorContent's auto-unwrap
// receives the wrong thing.
const wsBase = config.public.apiBase.replace(/^http/, 'ws')
const wsParams: Record<string, string> = {}
if (auth.accessToken) wsParams.token = auth.accessToken
if (props.linkToken) wsParams.link = props.linkToken
const provider = new WebsocketProvider(`${wsBase}/ws/pages`, props.pageId, doc, { params: wsParams })
const currentUser = {
  name: auth.user?.email ?? 'Anonymous',
  color: userColor(auth.user?.id ?? props.pageId),
}

const editor = useEditor({
  extensions: [
    StarterKit.configure({ undoRedo: false }),
    Image,
    TextStyle,
    Color,
    Highlight.configure({ multicolor: true }),
    TaskList,
    TaskItem.configure({ nested: true }),
    Collaboration.configure({ document: doc }),
    CollaborationCaret.configure({ provider, user: currentUser }),
    SlashCommand.configure({ onInsertImage: triggerImagePicker }),
  ],
  editorProps: {
    attributes: { class: 'outline-none min-h-[60vh]' },
    handlePaste(_view, event) {
      const files = imagesFrom(event.clipboardData?.items)
      if (files.length === 0) return false
      event.preventDefault()
      insertImages(files)
      return true
    },
    handleDrop(_view, event) {
      const files = imagesFrom((event as DragEvent).dataTransfer?.files)
      if (files.length === 0) return false
      event.preventDefault()
      insertImages(files)
      return true
    },
  },
  onUpdate: scheduleSave,
})

onMounted(() => {
  // Loaded async and merged into the already-live doc rather than blocking
  // editor creation on it; Yjs updates are commutative, so ordering
  // relative to the websocket provider's own sync doesn't matter.
  api<ArrayBuffer>(`/pages/${props.pageId}/content`, {
    responseType: 'arrayBuffer',
    query: props.linkToken ? { link: props.linkToken } : undefined,
  }).then((bytes) => {
    if (bytes && bytes.byteLength > 0) Y.applyUpdate(doc, new Uint8Array(bytes))
  })
})

onBeforeUnmount(() => {
  clearTimeout(saveTimer)
  clearTimeout(titleTimer)
  provider.destroy()
  doc.destroy()
})
</script>

<template>
  <div class="mx-auto max-w-3xl">
    <input
      v-model="titleDraft"
      type="text"
      placeholder="Untitled"
      class="mb-2 w-full border-none bg-transparent text-4xl font-bold text-slate-900 outline-none placeholder:text-slate-300 dark:text-slate-100 dark:placeholder:text-slate-700"
      @input="scheduleTitleSave"
      @blur="pagesStore.updatePage(pageId, { title: titleDraft || 'Untitled' })"
    />

    <input ref="imageInput" type="file" accept="image/*" class="hidden" @change="onImageInputChange" />

    <div v-if="editor" class="group/editor relative pl-8">
      <DragHandle
        :editor="editor"
        :on-node-change="onDragHandleNodeChange"
        :compute-position-config="{
          placement: 'left-start',
          strategy: 'absolute',
          // Sit just left of the block; height matches line-height so the
          // grip is vertically centered on the first text line.
          offset: { mainAxis: 6, crossAxis: 0 },
        }"
      >
        <button
          type="button"
          aria-label="Open block menu"
          class="flex h-[1.75rem] w-5 cursor-grab items-center justify-center rounded text-sm leading-none text-slate-400 opacity-0 hover:bg-slate-100 group-hover/editor:opacity-100 dark:text-slate-500 dark:hover:bg-slate-800"
          @click="openBlockMenu"
        >
          ⠿
        </button>
      </DragHandle>

      <BubbleMenu :editor="editor">
        <div class="flex items-center gap-0.5 rounded-md border border-slate-200 bg-white p-1 shadow-lg dark:border-slate-700 dark:bg-slate-900">
          <button
            type="button"
            class="rounded px-2 py-1 text-sm font-bold text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            :class="editor.isActive('bold') ? 'bg-slate-100 dark:bg-slate-800' : ''"
            @click="editor.chain().focus().toggleBold().run()"
          >
            B
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 text-sm italic text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            :class="editor.isActive('italic') ? 'bg-slate-100 dark:bg-slate-800' : ''"
            @click="editor.chain().focus().toggleItalic().run()"
          >
            I
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 text-sm line-through text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            :class="editor.isActive('strike') ? 'bg-slate-100 dark:bg-slate-800' : ''"
            @click="editor.chain().focus().toggleStrike().run()"
          >
            S
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 font-mono text-sm text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            :class="editor.isActive('code') ? 'bg-slate-100 dark:bg-slate-800' : ''"
            @click="editor.chain().focus().toggleCode().run()"
          >
            &lt;/&gt;
          </button>
          <div class="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
          <button
            v-for="c in textColors"
            :key="c.name"
            type="button"
            :title="c.name"
            class="h-5 w-5 rounded text-xs font-semibold hover:bg-slate-100 dark:hover:bg-slate-800"
            :style="{ color: c.value ?? 'inherit' }"
            @click="setSelectionColor(c.value)"
          >
            A
          </button>
        </div>
      </BubbleMenu>

      <EditorContent
        :editor="editor"
        class="text-base leading-7 text-slate-800 dark:text-slate-200 [&_h1]:mt-6 [&_h1]:text-3xl [&_h1]:font-bold [&_h2]:mt-5 [&_h2]:text-2xl [&_h2]:font-semibold [&_h3]:mt-4 [&_h3]:text-xl [&_h3]:font-semibold [&_p]:my-3 [&_ul]:my-3 [&_ul]:list-disc [&_ul]:pl-6 [&_ol]:my-3 [&_ol]:list-decimal [&_ol]:pl-6 [&_blockquote]:border-l-4 [&_blockquote]:border-slate-300 dark:[&_blockquote]:border-slate-700 [&_blockquote]:pl-4 [&_blockquote]:text-slate-600 dark:[&_blockquote]:text-slate-400 [&_img]:my-3 [&_img]:max-w-full [&_img]:rounded [&_.ProseMirror-selectednode]:rounded [&_.ProseMirror-selectednode]:bg-blue-50 dark:[&_.ProseMirror-selectednode]:bg-blue-950/40"
      />
    </div>
    <p v-else class="text-slate-400 dark:text-slate-500">Loading editor…</p>

    <BlockMenu
      v-if="blockMenu"
      :editor="editor"
      :pos="blockMenu.pos"
      :node="blockMenu.node"
      :x="blockMenu.x"
      :y="blockMenu.y"
      @close="blockMenu = null"
    />
  </div>
</template>
