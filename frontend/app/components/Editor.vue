<script setup lang="ts">
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import Collaboration from '@tiptap/extension-collaboration'
import CollaborationCaret from '@tiptap/extension-collaboration-caret'

const props = defineProps<{ pageId: string; workspaceId: string }>()

const api = useApi()
const auth = useAuthStore()
const config = useRuntimeConfig()
const minioBase = config.public.minioBase

const doc = new Y.Doc()
const editor = shallowRef<ReturnType<typeof useEditor> | null>(null)
let provider: WebsocketProvider | undefined

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

onMounted(async () => {
  const bytes = await api<ArrayBuffer>(`/pages/${props.pageId}/content`, {
    responseType: 'arrayBuffer',
  })
  if (bytes && bytes.byteLength > 0) Y.applyUpdate(doc, new Uint8Array(bytes))

  // Live collaboration over the backend's y-websocket-compatible endpoint.
  // Token goes in the query string because browsers can't set WS handshake headers.
  const wsBase = config.public.apiBase.replace(/^http/, 'ws')
  const currentUser = {
    name: auth.user?.email ?? 'Anonymous',
    color: userColor(auth.user?.id ?? props.pageId),
  }
  provider = new WebsocketProvider(`${wsBase}/ws/pages`, props.pageId, doc, {
    params: auth.accessToken ? { token: auth.accessToken } : {},
  })

  editor.value = useEditor({
    extensions: [
      StarterKit.configure({ undoRedo: false }),
      Image,
      Collaboration.configure({ document: doc }),
      CollaborationCaret.configure({ provider, user: currentUser }),
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
})

onBeforeUnmount(() => {
  clearTimeout(saveTimer)
  editor.value?.destroy()
  provider?.destroy()
  doc.destroy()
})
</script>

<template>
  <div class="mx-auto max-w-3xl">
    <EditorContent
      v-if="editor"
      :editor="editor"
      class="text-base leading-7 text-slate-800 dark:text-slate-200 [&_h1]:mt-6 [&_h1]:text-3xl [&_h1]:font-bold [&_h2]:mt-5 [&_h2]:text-2xl [&_h2]:font-semibold [&_h3]:mt-4 [&_h3]:text-xl [&_h3]:font-semibold [&_p]:my-3 [&_ul]:my-3 [&_ul]:list-disc [&_ul]:pl-6 [&_ol]:my-3 [&_ol]:list-decimal [&_ol]:pl-6 [&_blockquote]:border-l-4 [&_blockquote]:border-slate-300 dark:[&_blockquote]:border-slate-700 [&_blockquote]:pl-4 [&_blockquote]:text-slate-600 dark:[&_blockquote]:text-slate-400 [&_pre]:my-3 [&_pre]:rounded [&_pre]:bg-slate-900 [&_pre]:p-3 [&_pre]:text-sm [&_pre]:text-slate-100 [&_img]:my-3 [&_img]:max-w-full [&_img]:rounded"
    />
    <p v-else class="text-slate-400 dark:text-slate-500">Loading editor…</p>
  </div>
</template>
