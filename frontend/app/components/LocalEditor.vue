<script setup lang="ts">
import type { Editor } from '@tiptap/vue-3'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import { DragHandle } from '@tiptap/extension-drag-handle-vue-3'
import { useBlockMenu } from '~/composables/useBlockMenu'
import { bodyArrowUpToTitle, bodyBackspaceToTitle, docSelectAllToEditor, titleKeydown } from '~/utils/title'

const model = defineModel<string>({ required: true })

// The offline document's title, owned by local.vue via v-model:title and
// rendered here (replacing the page-level input it used to host). `title`
// doubles as the Save As filename there, and write-through follows the same
// `watch(model)` + autosave path as content edits.
const title = defineModel<string>('title')
const titleInput = ref<HTMLTextAreaElement | null>(null)

// Grow the title textarea to fit a long, wrapped title (see Editor.vue's
// autoGrowTitle for the same helper).
function autoGrowTitle() {
  const el = titleInput.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${el.scrollHeight}px`
}
watch(title, () => nextTick(autoGrowTitle))

// Same mechanism Editor.vue uses to hand its Tiptap instance up to its page
// shell — local.vue needs the live editor's `getJSON()` (not just HTML) to
// serialize to Markdown the way the server-backed export does.
const emit = defineEmits<{ 'editor-ready': [editor: Editor | null] }>()

// Images live inside the .html file as data URLs — an offline document has no
// server to upload attachments to, and a file that points at one wouldn't be
// self-contained.
function readAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(file)
  })
}

async function insertImages(files: File[]) {
  for (const file of files) {
    editor.value?.chain().focus().setImage({ src: await readAsDataUrl(file) }).run()
  }
}

const imageInput = ref<HTMLInputElement | null>(null)
function triggerImagePicker() {
  imageInput.value?.click()
}
function onImageInputChange(e: Event) {
  const input = e.target as HTMLInputElement
  const files = Array.from(input.files ?? [])
  if (files.length) insertImages(files)
  input.value = ''
}

const editor = useEditor({
  content: model.value,
  extensions: useEditorExtensions({ onInsertImage: triggerImagePicker }),
  editorProps: {
    // pl-12 gives the left gutter room for the drag-handle (see useBlockMenu);
    // without it the handle floats over the text instead of beside the block.
    attributes: { class: 'outline-none min-h-[60vh] pl-12' },
    handlePaste(_view, event) {
      const files = Array.from(event.clipboardData?.items ?? [])
        .map((it) => it.getAsFile())
        .filter((f): f is File => !!f && f.type.startsWith('image/'))
      if (files.length === 0) return false
      event.preventDefault()
      insertImages(files)
      return true
    },
    handleDrop(_view, event) {
      const files = Array.from((event as DragEvent).dataTransfer?.files ?? []).filter((f) =>
        f.type.startsWith('image/'),
      )
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
        title: title.value ?? '',
        onTitle: (t) => (title.value = t),
      })
    },
  },
  onUpdate: ({ editor: e }) => {
    model.value = e.getHTML()
  },
})

// Parent replaces the whole document on New/Open; guard against echoing our own
// onUpdate back in, which would reset the cursor on every keystroke.
watch(model, (html) => {
  if (editor.value && html !== editor.value.getHTML()) {
    editor.value.commands.setContent(html, { emitUpdate: false })
  }
})

// Ctrl/Cmd+A with focus on the page shell (buttons, links, or nothing) would
// let the browser select every text node — sidebar and topbar included. Route
// it to the editor's own selectAll so only the document body is selected;
// focus on the title input or the contenteditable keeps native behavior (the
// input's @keydown handles its own select).
function onDocKeydown(e: KeyboardEvent) {
  docSelectAllToEditor(e, editor.value)
}
onMounted(() => document.addEventListener('keydown', onDocKeydown))
onBeforeUnmount(() => document.removeEventListener('keydown', onDocKeydown))

watch(editor, (e) => emit('editor-ready', e ?? null), { immediate: true })

// Gutter drag-handle + right-click/selection block menu — same behavior as
// the collaborative editor, minus the server-backed pieces (comments, AI).
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
  onDragHandleNodeChange,
} = useBlockMenu(editor)
</script>

<template>
  <div class="w-full">
    <input ref="imageInput" type="file" accept="image/*" class="hidden" @change="onImageInputChange" />

    <textarea
      ref="titleInput"
      v-model="title"
      rows="1"
      placeholder="Untitled"
      class="mb-2 block w-full resize-none border-none bg-transparent text-4xl font-bold leading-tight text-neutral-900 outline-none placeholder:text-neutral-300 dark:text-neutral-100 dark:placeholder:text-neutral-700"
      @input="autoGrowTitle"
      @keydown="titleKeydown($event, { editor, input: titleInput, onTitle: (t) => { title = t; nextTick(autoGrowTitle) } })"
    />

    <div
      v-if="editor"
      class="group/editor relative"
      @contextmenu="openBlockMenuFromContextMenu"
      @mouseup="onEditorMouseUp"
      @mousemove="onEditorMouseMove"
    >
      <DragHandle
        :editor="editor"
        :on-node-change="onDragHandleNodeChange"
        :compute-position-config="dragHandlePositionConfig"
      >
        <div class="flex items-center gap-0.5" :style="handleBoxStyle">
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
      :selection-range="blockMenu.selectionRange"
      @close="blockMenu = null"
    />
  </div>
</template>
