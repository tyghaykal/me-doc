import type { Editor } from '@tiptap/vue-3'

// The utils take the unwrapped Editor instance (null before ready). Callers
// that hold the ref from useEditor() pass `editor.value`; template callers
// pass `editor` (auto-unwrapped).

// Focus the document body, moving the caret to the first block's start (or the
// start of the document). Assumes the caller has already asked the title input
// to blur so a subsequent Ctrl+A can't re-select the title's own text.
export function focusBodyStart(editor: Editor | null): void {
  editor?.commands.focus('start')
}

// Take the first non-empty line of an editor doc and make it the page title
// (reported through `onTitle`). The line is removed from the doc via a normal
// deleteRange, so the change stays undoable and syncs through the collab
// provider like any other edit. Returns false when there's nothing to promote
// (empty doc, or the first block is a heading/list/etc. — a backspace there is
// just a normal delete, not a title-promotion).
export function pullTitleFromDoc(editor: Editor | null, onTitle: (title: string) => void): boolean {
  if (!editor) return false

  const doc = editor.state.doc
  if (doc.childCount === 0) return false
  const first = doc.child(0)
  // Only plain paragraphs can promote — a real heading is already a title.
  if (first.type.name !== 'paragraph') return false

  const text = first.textContent.trim()
  if (!text) return false

  onTitle(text)
  // Range [1, 1+nodeSize) is exactly the first block (doc content starts at
  // pos 1). Delete it, then drop the caret to the start of what remains.
  editor.chain().deleteRange({ from: 1, to: 1 + first.nodeSize }).focus('start').run()
  return true
}

// Title-input key handling, shared by the collab Editor and the offline local
// editor: Enter/Tab/ArrowDown leave the title and focus the body's first line;
// Ctrl/Cmd+A inside the title selects just the title text (the input's own
// value), never the whole page. Backspace on an empty title with a non-empty
// body promotes the body's first line into the title.
export function titleKeydown(
  ev: KeyboardEvent,
  opts: { editor: Editor | null; input: HTMLInputElement | null | undefined; onTitle: (t: string) => void },
): void {
  const { editor, input, onTitle } = opts

  if (ev.key === 'Enter' || ev.key === 'Tab') {
    ev.preventDefault()
    input?.blur()
    focusBodyStart(editor)
    return
  }

  if (ev.key === 'ArrowDown') {
    // Only leave when the caret is at the end of the title text (a mid-title
    // ArrowDown should still move the caret within the field).
    if (input && input.selectionStart === input.value.length) {
      ev.preventDefault()
      input.blur()
      focusBodyStart(editor)
    }
    return
  }

  if (ev.key === 'Backspace' && input && input.value === '') {
    ev.preventDefault()
    if (pullTitleFromDoc(editor, onTitle)) {
      input.blur()
      focusBodyStart(editor)
    }
  }
}

// Editor-body key handling: with an empty title, a Backspace at the very start
// of the document promotes the first line into the title instead of being a
// no-op. Wire into the editor's handleKeyDown and return its result (true =
// handled, so ProseMirror doesn't also process the Backspace).
export function bodyBackspaceToTitle(
  ev: KeyboardEvent,
  opts: { editor: Editor | null; title: string; onTitle: (t: string) => void },
): boolean {
  if (ev.key !== 'Backspace') return false
  if (opts.title !== '') return false
  if (!opts.editor) return false
  const { selection } = opts.editor.state
  // Caret at the absolute start of the document (before the first block).
  if (!selection.empty || selection.$from.pos !== 1) return false
  ev.preventDefault()
  return pullTitleFromDoc(opts.editor, opts.onTitle)
}

// Editor-body key handling: with the caret at the very first line, Arrow Up
// moves focus into the title input (caret at the end of the title text),
// mirroring the title→body Arrow Down the other way. Wire into the editor's
// handleKeyDown; returns true when it handles the keypress.
export function bodyArrowUpToTitle(
  ev: KeyboardEvent,
  opts: { editor: Editor | null; input: HTMLInputElement | null | undefined },
): boolean {
  if (ev.key !== 'ArrowUp') return false
  if (!opts.editor) return false
  const { selection } = opts.editor.state
  // Caret at the absolute start of the document (before the first block).
  if (!selection.empty || selection.$from.pos !== 1) return false
  ev.preventDefault()
  opts.input?.focus()
  // Put the caret at the end of the title so typing continues from where the
  // title left off; a fresh empty title just gets the caret in place.
  const end = opts.input?.value.length ?? 0
  opts.input?.setSelectionRange(end, end)
  return true
}

// Document-level Ctrl+A redirect. When focus is on the page shell (a button,
// a link, or nothing at all) the browser's default select-all grabs every
// text node on the page — sidebar and topbar included. If the target isn't an
// editable element, route the shortcut to the editor's own selectAll instead
// so only the document body is selected. Editable targets (the title input,
// the ProseMirror contenteditable) keep their native behavior.
export function docSelectAllToEditor(ev: KeyboardEvent, editor: Editor | null): void {
  if (!(ev.ctrlKey || ev.metaKey) || ev.key.toLowerCase() !== 'a') return
  const target = ev.target as HTMLElement | null
  if (target?.closest('input, textarea, [contenteditable="true"], [contenteditable="plaintext-only"]')) return
  ev.preventDefault()
  editor?.chain().focus().selectAll().run()
}
