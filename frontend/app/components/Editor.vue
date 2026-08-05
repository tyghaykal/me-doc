<script setup lang="ts">
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import { EditorContent, useEditor, type Editor } from '@tiptap/vue-3'
import { BubbleMenu } from '@tiptap/vue-3/menus'
import { DragHandle } from '@tiptap/extension-drag-handle-vue-3'
import Collaboration from '@tiptap/extension-collaboration'
import CollaborationCaret from '@tiptap/extension-collaboration-caret'
import { useBlockMenu } from '~/composables/useBlockMenu'
import { DEFAULT_PAGE_ICON, PAGE_ICON_CHOICES } from '~/stores/pages'
import { bodyArrowUpToTitle, bodyBackspaceToTitle, docSelectAllToEditor, titleKeydown } from '~/utils/title'

const props = defineProps<{
  pageId: string
  workspaceId: string
  title: string
  icon?: string | null
  linkToken?: string | null
  readOnly?: boolean
}>()

const emit = defineEmits<{
  'presence-change': [users: { clientId: number; name: string; email: string | null; color: string; avatarUrl: string | null }[]]
  'editor-ready': [editor: Editor | null]
  'open-comment': [markId: string]
}>()

const api = useApi()
const auth = useAuthStore()
const pagesStore = usePagesStore()
const config = useRuntimeConfig()

const doc = new Y.Doc()

// --- Title ---
const titleDraft = ref(props.title)
const titleInput = ref<HTMLTextAreaElement | null>(null)
watch(
  () => props.title,
  (t) => {
    titleDraft.value = t
  },
)
let titleTimer: ReturnType<typeof setTimeout> | undefined

// The title is a <textarea> so a long title soft-wraps onto multiple lines
// instead of scrolling horizontally in a single-line input. rows="1" keeps
// the initial height a single line; this grows the element to fit its content
// (and shrinks back when text is removed). scrollHeight is clamped to the
// content's natural height so it never counts the row below the last line.
function autoGrowTitle() {
  const el = titleInput.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${el.scrollHeight}px`
}

// A long title that already exists (opened page, restored version) must be
// expanded when the textarea first renders — the v-model doesn't fire @input.
// `immediate` covers the very first render where a long stored title would
// otherwise stay clipped to rows="1".
watch(
  () => titleDraft.value,
  () => nextTick(autoGrowTitle),
  { immediate: true },
)

function scheduleTitleSave() {
  clearTimeout(titleTimer)
  titleTimer = setTimeout(() => {
    pagesStore.updatePage(props.pageId, { title: titleDraft.value }, props.linkToken)
  }, 800)
}

// --- Icon ---
const iconDraft = ref(props.icon ?? '')
const iconPickerOpen = ref(false)
watch(
  () => props.icon,
  (i) => {
    iconDraft.value = i ?? ''
  },
)
function setIcon(icon: string | null) {
  iconDraft.value = icon ?? ''
  iconPickerOpen.value = false
  pagesStore.updatePage(props.pageId, { icon }, props.linkToken)
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
      body: { workspace_id: props.workspaceId, filename: file.name, content_type: file.type, size: file.size },
    },
  )
  await fetch(upload_url, { method: 'PUT', body: file, headers: { 'Content-Type': file.type } })
  return resolveAttachmentUrl(s3_key)
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

// A copy source pasting genuine markdown text still leaves the literal
// syntax characters (##, **, [^1], :::, ...) in the plain-text clipboard
// entry — a true rich-text/webpage copy's plain-text fallback is just
// rendered prose with none of that. Sniffing content is more reliable than
// gating on the presence of a text/html entry: many sources (VS Code,
// browsers, some OS clipboard managers) attach one even for what looks like
// a plain copy, which previously made markdown-paste silently never fire.
function looksLikeMarkdown(text: string): boolean {
  return (
    /(^|\n)[ \t]{0,3}(#{1,6}[ \t]|[-*+][ \t]|\d+\.[ \t]|>[ \t]|```|~~~|:::|\[\^[^\]]+\]:|\*\[[^\]]+\]:|[ \t]{0,3}[:~][ \t])/.test(
      text,
    ) || /\*\*[^*\n]+\*\*|__[^_\n]+__|~~[^~\n]+~~|==[^=\n]+==|\+\+[^+\n]+\+\+|\[\^[^\]]+\]|\[[^\]]+\]\([^)]+\)/.test(
      text,
    )
  )
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

// --- Slash-command diagram embed (opens the diagram picker) ---
const diagramPickerOpen = ref(false)
function openDiagramPicker() {
  diagramPickerOpen.value = true
}
function insertDiagramEmbed(diagram: { id: string; title: string }) {
  diagramPickerOpen.value = false
  editor.value
    ?.chain()
    .focus()
    .insertContent({ type: 'diagramEmbed', attrs: { diagramId: diagram.id, title: diagram.title } })
    .run()
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
  name: auth.user?.display_name || auth.user?.email || 'Anonymous',
  email: auth.user?.email ?? null,
  color: userColor(auth.user?.id ?? props.pageId),
  avatarUrl: resolveAvatarUrl(auth.user?.avatar_key),
}

// Presence list: who else is viewing/editing right now. Read directly from
// awareness (editor.storage.collaborationCaret.users isn't Vue-reactive).
// email rides along so the topbar can dedupe a peer against the signed-in
// user (e.g. the same account open in two tabs/windows) instead of showing
// both the "you" chip and a second identical presence avatar.
function updatePresence() {
  const users = Array.from(provider.awareness.getStates().entries())
    .filter(([clientId]) => clientId !== provider.awareness.clientID)
    .map(([clientId, state]: [number, any]) => ({
      clientId,
      name: state?.user?.name ?? 'Anonymous',
      email: state?.user?.email ?? null,
      color: state?.user?.color ?? '#999999',
      avatarUrl: state?.user?.avatarUrl ?? null,
    }))
  emit('presence-change', users)
}
provider.awareness.on('change', updatePresence)

const editor = useEditor({
  editable: !props.readOnly,
  extensions: [
    // Yjs owns the undo stack here, so StarterKit's own history is off.
    ...useEditorExtensions({
      undoRedo: false,
      onInsertImage: triggerImagePicker,
      onEmbedDiagram: openDiagramPicker,
      comments: true,
    }),
    Collaboration.configure({ document: doc }),
    CollaborationCaret.configure({ provider, user: currentUser }),
  ],
  editorProps: {
    // pl-12 lives here (on .ProseMirror itself), not on the wrapper div below,
    // so that box's own geometry covers the left gutter where the drag-handle
    // sits. @tiptap/extension-drag-handle only listens for mousemove on
    // editor.view.dom (.ProseMirror) — if the gutter were the wrapper's own
    // padding instead, that space would sit outside .ProseMirror's bounding
    // box entirely and hovering it would never dispatch through the plugin's
    // listener, so the handle would only ever appear once the pointer reached
    // actual text. The plugin's own clampToContent() already resolves gutter
    // coordinates to the nearest block's real edge, so this is a pure
    // ownership fix — text position is unchanged.
    attributes: { class: 'outline-none min-h-[60vh] pl-12' },
    handlePaste(_view, event) {
      const files = imagesFrom(event.clipboardData?.items)
      if (files.length > 0) {
        event.preventDefault()
        insertImages(files)
        return true
      }

      // Only intervene when the plain text actually looks like markdown
      // source — a genuine rich webpage/doc paste still goes through
      // ProseMirror's normal HTML handling either way.
      const text = event.clipboardData?.getData('text/plain')
      if (text && looksLikeMarkdown(text)) {
        event.preventDefault()
        editor.value?.chain().focus().insertContent(markdownToHtml(text)).run()
        return true
      }
      return false
    },
    handleDrop(_view, event) {
      const files = imagesFrom((event as DragEvent).dataTransfer?.files)
      if (files.length === 0) return false
      event.preventDefault()
      insertImages(files)
      return true
    },
    handleKeyDown(_view, event) {
      // Arrow Up at the very first line moves the caret into the title input.
      if (bodyArrowUpToTitle(event, { editor: editor.value, input: titleInput.value })) return true
      // Backspace at the very start of an empty-titled doc promotes the first
      // line into the title. Returns true so ProseMirror drops the keypress.
      return bodyBackspaceToTitle(event, {
        editor: editor.value,
        title: titleDraft.value,
        onTitle: (t) => {
          titleDraft.value = t
          scheduleTitleSave()
        },
      })
    },
  },
  onUpdate: scheduleSave,
})

// Gutter drag-handle + right-click/selection block menu — shared with the
// offline LocalEditor.vue via useBlockMenu (see that file for the comments
// explaining the handle geometry/hover quirks). Must run after `editor` above,
// which it closes over.
const {
  hoveredBlock,
  blockMenu,
  handleBoxStyle,
  dragHandlePositionConfig,
  insertBlockBelow,
  openBlockMenuFromGrip,
  openBlockMenuFromContextMenu,
  onEditorMouseMove,
  onEditorMouseUp,
  openBlockMenuFromSelection,
  onDragHandleNodeChange,
} = useBlockMenu(editor, { readOnly: () => props.readOnly })

// Parent (page shell) mounts the right-side TOC from this instance.
watch(
  editor,
  (e) => emit('editor-ready', e ?? null),
  { immediate: true },
)

// Redirect Ctrl/Cmd+A (see onMounted) into the editor's own selectAll, then
// pop the same block menu a mouse drag-selection would (docSelectAllToEditor
// only preventDefault()s when it actually redirected the selection, so that
// flags whether this keypress was the one we care about).
function onDocKeydown(e: KeyboardEvent) {
  docSelectAllToEditor(e, editor.value)
  if (e.defaultPrevented) openBlockMenuFromSelection()
}

// Click a comment highlight → open that thread in the sidebar.
function onEditorClick(e: MouseEvent) {
  const t = e.target as HTMLElement | null
  const mark = t?.closest?.('[data-comment-id], .comment-anchor') as HTMLElement | null
  if (!mark) return
  const id = mark.getAttribute('data-comment-id')
  if (id) emit('open-comment', id)
}

onMounted(() => {
  // Ctrl/Cmd+A with focus on the page shell (buttons, links, or nothing) would
  // let the browser select every text node — sidebar and topbar included.
  // Route it to the editor's own selectAll so only the document body is
  // selected; focus on the title input or the contenteditable keeps native
  // behavior (the title input's @keydown handles its own select).
  document.addEventListener('keydown', onDocKeydown)

  // Loaded async and merged into the already-live doc rather than blocking
  // editor creation on it; Yjs updates are commutative, so ordering
  // relative to the websocket provider's own sync doesn't matter.
  api<ArrayBuffer>(`/pages/${props.pageId}/content`, {
    responseType: 'arrayBuffer',
    query: props.linkToken ? { link: props.linkToken } : undefined,
  }).then((bytes) => {
    if (bytes && bytes.byteLength > 0) Y.applyUpdate(doc, new Uint8Array(bytes))
  })

  // A page just created from an imported .txt/.md file (PageTree.vue's
  // "Import" button) — apply its converted content now that this fresh
  // page's editor/doc exist.
  const importedHtml = pagesStore.takePendingImport(props.pageId)
  if (importedHtml) editor.value?.commands.setContent(importedHtml)
})

onBeforeUnmount(() => {
  clearTimeout(saveTimer)
  clearTimeout(titleTimer)
  document.removeEventListener('keydown', onDocKeydown)
  provider.awareness.off('change', updatePresence)
  provider.destroy()
  doc.destroy()
  emit('editor-ready', null)
})
</script>

<template>
  <div class="w-full">
    <div class="relative mb-1 inline-block">
      <button
        type="button"
        :disabled="readOnly"
        class="rounded px-1 text-4xl leading-none enabled:hover:bg-neutral-100 dark:enabled:hover:bg-neutral-800"
        @click="iconPickerOpen = !iconPickerOpen"
      >
        {{ iconDraft || DEFAULT_PAGE_ICON }}
      </button>

      <template v-if="iconPickerOpen && !readOnly">
        <div class="fixed inset-0 z-40" @click="iconPickerOpen = false" />
        <div class="absolute left-0 z-50 mt-1 grid w-64 grid-cols-8 gap-1 rounded-md border border-neutral-200 bg-white p-2 shadow-lg dark:border-neutral-700 dark:bg-neutral-900">
          <button
            v-for="e in PAGE_ICON_CHOICES"
            :key="e"
            type="button"
            class="rounded p-1 text-xl hover:bg-neutral-100 dark:hover:bg-neutral-800"
            @click="setIcon(e)"
          >
            {{ e }}
          </button>
          <button
            v-if="iconDraft"
            type="button"
            class="col-span-8 mt-1 rounded px-2 py-1 text-left text-xs text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
            @click="setIcon(null)"
          >
            Remove icon
          </button>
        </div>
      </template>
    </div>

    <textarea
      ref="titleInput"
      v-model="titleDraft"
      rows="1"
      placeholder="Untitled"
      :readonly="readOnly"
      class="mb-2 block w-full resize-none border-none bg-transparent text-4xl font-bold leading-tight text-neutral-900 outline-none placeholder:text-neutral-300 dark:text-neutral-100 dark:placeholder:text-neutral-700"
      @input="!readOnly && (autoGrowTitle(), scheduleTitleSave())"
      @blur="!readOnly && pagesStore.updatePage(pageId, { title: titleDraft }, linkToken)"
      @keydown="!readOnly && titleKeydown($event, { editor, input: titleInput, onTitle: (t) => { titleDraft = t; nextTick(autoGrowTitle); scheduleTitleSave() } })"
    />

    <input ref="imageInput" type="file" accept="image/*" class="hidden" @change="onImageInputChange" />

    <DiagramPicker
      v-if="diagramPickerOpen"
      :workspace-id="workspaceId"
      @pick="insertDiagramEmbed"
      @close="diagramPickerOpen = false"
    />

    <div
      v-if="editor"
      class="group/editor relative"
      @click="onEditorClick"
      @contextmenu="openBlockMenuFromContextMenu"
      @mouseup="onEditorMouseUp"
      @mousemove="onEditorMouseMove"
    >
      <DragHandle
        v-if="!readOnly"
        :editor="editor"
        :on-node-change="onDragHandleNodeChange"
        :compute-position-config="dragHandlePositionConfig"
      >
        <div
          class="flex items-center gap-0.5"
          :style="handleBoxStyle"
        >
          <button
            type="button"
            aria-label="Add block below"
            title="Add block below"
            class="flex h-full w-5 items-center justify-center rounded text-sm leading-none text-neutral-400 opacity-0 hover:bg-neutral-100 group-hover/editor:opacity-100 dark:text-neutral-500 dark:hover:bg-neutral-800"
            @click="insertBlockBelow"
          >
            +
          </button>
          <button
            type="button"
            aria-label="Open block menu"
            class="flex h-full w-5 cursor-grab items-center justify-center rounded text-sm leading-none text-neutral-400 opacity-0 hover:bg-neutral-100 group-hover/editor:opacity-100 dark:text-neutral-500 dark:hover:bg-neutral-800"
            @click="openBlockMenuFromGrip"
          >
            ⠿
          </button>
        </div>
      </DragHandle>

      <BubbleMenu
        v-if="!readOnly"
        :editor="editor"
        :plugin-key="'tableBubbleMenu'"
        :should-show="({ editor: e }: any) => e.isActive('table')"
      >
        <div class="flex items-center gap-0.5 rounded-md border border-neutral-200 bg-white p-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900">
          <button
            type="button"
            title="Add column before"
            class="rounded px-2 py-1 text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().addColumnBefore().run()"
          >
            +Col←
          </button>
          <button
            type="button"
            title="Add column after"
            class="rounded px-2 py-1 text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().addColumnAfter().run()"
          >
            +Col→
          </button>
          <button
            type="button"
            title="Delete column"
            class="rounded px-2 py-1 text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().deleteColumn().run()"
          >
            −Col
          </button>
          <div class="mx-1 h-4 w-px bg-neutral-200 dark:bg-neutral-700" />
          <button
            type="button"
            title="Add row before"
            class="rounded px-2 py-1 text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().addRowBefore().run()"
          >
            +Row↑
          </button>
          <button
            type="button"
            title="Add row after"
            class="rounded px-2 py-1 text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().addRowAfter().run()"
          >
            +Row↓
          </button>
          <button
            type="button"
            title="Delete row"
            class="rounded px-2 py-1 text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().deleteRow().run()"
          >
            −Row
          </button>
          <div class="mx-1 h-4 w-px bg-neutral-200 dark:bg-neutral-700" />
          <button
            type="button"
            title="Delete table"
            class="rounded px-2 py-1 text-xs text-red-600 hover:bg-neutral-100 dark:text-red-400 dark:hover:bg-neutral-800"
            @click="editor.chain().focus().deleteTable().run()"
          >
            Delete table
          </button>
        </div>
      </BubbleMenu>

      <EditorContent
        :editor="editor"
        class="text-base leading-7 text-neutral-800 dark:text-neutral-200 [&_p]:my-3 [&_ul]:my-3 [&_ul]:list-disc [&_ul]:pl-6 [&_ol]:my-3 [&_ol]:list-decimal [&_ol]:pl-6 [&_blockquote]:border-l-4 [&_blockquote]:border-neutral-300 dark:[&_blockquote]:border-neutral-700 [&_blockquote]:pl-4 [&_blockquote]:text-neutral-600 dark:[&_blockquote]:text-neutral-400 [&_img]:my-3 [&_img]:max-w-full [&_img]:rounded [&_.ProseMirror-selectednode]:rounded [&_.ProseMirror-selectednode]:bg-neutral-100 dark:[&_.ProseMirror-selectednode]:bg-neutral-800/40"
      />
    </div>
    <p v-else class="text-neutral-400 dark:text-neutral-500">Loading editor…</p>

    <BlockMenu
      v-if="blockMenu"
      :editor="editor"
      :pos="blockMenu.pos"
      :node="blockMenu.node"
      :x="blockMenu.x"
      :y="blockMenu.y"
      :page-id="pageId"
      :workspace-id="workspaceId"
      :selection-range="blockMenu.selectionRange"
      @close="blockMenu = null"
    />
  </div>
</template>
