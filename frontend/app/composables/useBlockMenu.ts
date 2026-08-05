import { NodeSelection } from '@tiptap/pm/state'
import { useEditor, type Editor } from '@tiptap/vue-3'

export type HoveredBlock = { node: any; pos: number }
export type BlockMenuState = {
  x: number
  y: number
  node: any
  pos: number
  selectionRange?: { from: number; to: number }
}

// Drag-handle gutter + right-click/selection block menu, shared by the
// collaborative Editor.vue and the offline LocalEditor.vue. Editor.vue layers
// collab carets and comments on top; the menu/handle state itself is identical.
export function useBlockMenu(editor: ReturnType<typeof useEditor>, opts?: { readOnly?: () => boolean }) {
  const hoveredBlock = ref<HoveredBlock | null>(null)
  const blockMenu = ref<BlockMenuState | null>(null)

  // The drag-handle extension reports "no node" as soon as the pointer's exact
  // x,y stops resolving to the block's own DOM (e.g. drifting into the left
  // gutter to reach the grip, or past a short line's text into empty space on
  // the same row) — track the real pointer position and only actually clear
  // the hovered block once it's left that block's vertical row, not just its
  // horizontal bounds.
  const lastPointerY = ref(0)
  function onEditorMouseMove(e: MouseEvent) {
    lastPointerY.value = e.clientY
  }

  function onDragHandleNodeChange(data: { node: any; pos: number }) {
    if (data.node) {
      hoveredBlock.value = { node: data.node, pos: data.pos }
      updateHandleBoxHeight(data.pos)
      return
    }
    if (hoveredBlock.value && editor.value) {
      const dom = editor.value.view.nodeDOM(hoveredBlock.value.pos)
      if (dom instanceof HTMLElement) {
        const rect = dom.getBoundingClientRect()
        if (lastPointerY.value >= rect.top && lastPointerY.value <= rect.bottom) return
      }
    }
    hoveredBlock.value = null
  }

  // The +/⠿ buttons were a hardcoded h-[1.75rem] matching only the base
  // paragraph's leading-7 — any block with a different line-height (e.g. code
  // blocks at line-height:1.6/0.875rem in main.css, ~1.4rem) then centers
  // against the wrong box height and visibly drifts off the text line.
  // Read the hovered block's own computed line-height instead.
  const handleBoxHeight = ref('1.75rem')
  function updateHandleBoxHeight(pos: number) {
    const dom = editor.value?.view.nodeDOM(pos)
    if (!(dom instanceof HTMLElement)) return
    const lineHeight = getComputedStyle(dom).lineHeight
    handleBoxHeight.value = lineHeight && lineHeight !== 'normal' ? lineHeight : '1.75rem'
  }

  // No offset middleware here on purpose: the handle's wrapper element is
  // appended as a sibling of the ProseMirror content with pointer-events:none
  // (set by @tiptap/extension-drag-handle itself), and the library's own
  // mouseleave guard hides the handle whenever the cursor's relatedTarget
  // lands outside both the content and the handle. A horizontal gap here
  // becomes a dead zone the mouse must cross, so it never resolves to either
  // element and the handle hides before the click lands. Any left-gutter
  // spacing has to come from the button's own padding/margin instead, not
  // from moving the floating box away from the block edge.
  const dragHandlePositionConfig = {
    placement: 'left-start' as const,
    strategy: 'absolute' as const,
    middleware: [],
  }

  // @tiptap/extension-drag-handle hides its floating box via visibility/
  // pointer-events the instant the mouse leaves the ProseMirror content
  // element — including into our own left gutter, since its wrapper sits
  // outside that gutter and its mouseleave guard only checks
  // `wrapper.contains(relatedTarget)`. hoveredBlock above already tracks
  // "pointer is still over this block's row" correctly; visibility and
  // pointer-events both inherit down the DOM, so re-declaring them here on
  // our own element overrides the library's inline styles on its ancestor
  // with zero specificity fight, keeping the buttons visible and clickable
  // while hovered.
  const handleBoxStyle = computed(() => ({
    height: handleBoxHeight.value,
    visibility: hoveredBlock.value ? ('visible' as const) : undefined,
    pointerEvents: hoveredBlock.value ? ('auto' as const) : undefined,
  }))

  function insertBlockBelow() {
    if (!hoveredBlock.value || !editor.value) return
    const { pos, node } = hoveredBlock.value
    const insertPos = pos + node.nodeSize
    editor.value
      .chain()
      .focus()
      .insertContentAt(insertPos, { type: 'paragraph' })
      .setTextSelection(insertPos + 1)
      .run()
  }

  function openBlockMenu(x: number, y: number, selectionRange?: { from: number; to: number }) {
    if (!hoveredBlock.value || !editor.value) return
    const { node, pos } = hoveredBlock.value
    // Only collapse to a whole-block NodeSelection when there's no specific
    // text range to preserve — a marked selection keeps its own highlight.
    if (!selectionRange) editor.value.chain().focus().setNodeSelection(pos).run()
    blockMenu.value = { x, y, node, pos, selectionRange }
  }

  function openBlockMenuFromGrip(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const MENU_WIDTH = 256 // matches BlockMenu.vue's w-64
    openBlockMenu(Math.max(8, rect.left - MENU_WIDTH - 4), rect.top)
  }

  function activeTextSelection(): { from: number; to: number } | undefined {
    if (!editor.value) return undefined
    const { selection } = editor.value.state
    // A single click on an atom node (diagram, image, ...) resolves to a
    // NodeSelection, which is also non-empty — without this check it looked
    // identical to a drag-selected text range and popped the same menu a
    // right-click would (comment option included).
    if (selection.empty || selection instanceof NodeSelection || editor.value.isActive('table')) return undefined
    return { from: selection.from, to: selection.to }
  }

  // Right-click anywhere in the editor opens the same consolidated menu
  // (styling + comment + duplicate/delete) — falls through to the browser's
  // native context menu when no block is currently tracked as hovered.
  function openBlockMenuFromContextMenu(e: MouseEvent) {
    if (!hoveredBlock.value) return
    e.preventDefault()
    openBlockMenu(e.clientX, e.clientY, activeTextSelection())
  }

  // Marking (dragging to select) text shows the same menu automatically once
  // the drag finishes — checked on mouseup rather than every selection change
  // so it doesn't pop up mid-drag and fight the selection gesture.
  function onEditorMouseUp() {
    if (!editor.value || opts?.readOnly?.()) return
    const selectionRange = activeTextSelection()
    if (!selectionRange) return
    const coords = editor.value.view.coordsAtPos(selectionRange.to)
    openBlockMenu(coords.left, coords.bottom + 6, selectionRange)
  }

  // Same "selection just landed → show the menu" behavior as onEditorMouseUp,
  // for selections made without a mouse gesture to hook (Ctrl/Cmd+A). openBlockMenu()
  // requires a hoveredBlock, which the drag-handle's mousemove tracking never
  // set here since the pointer never touched a block — resolve one from the
  // selection's own start so duplicate/remove/append still have a real block
  // anchor. The AI/format actions don't need it: they scope off selectionRange.
  function openBlockMenuFromSelection() {
    if (!editor.value || opts?.readOnly?.()) return
    const selectionRange = activeTextSelection()
    if (!selectionRange) return
    if (!hoveredBlock.value) {
      const doc = editor.value.state.doc
      const $from = doc.resolve(selectionRange.from)
      const depth = $from.depth
      const node = depth > 0 ? $from.node(depth) : doc.firstChild
      const pos = depth > 0 ? $from.before(depth) : 0
      if (node) hoveredBlock.value = { node, pos }
    }
    const coords = editor.value.view.coordsAtPos(selectionRange.to)
    openBlockMenu(coords.left, coords.bottom + 6, selectionRange)
  }

  return {
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
  }
}
