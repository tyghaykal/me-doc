<script setup lang="ts">
import { useCommentsStore } from '~/stores/comments'

const props = defineProps<{
  editor: any
  pos: number
  node: any
  x: number
  y: number
  pageId?: string
  workspaceId?: string
  selectionRange?: { from: number; to: number } | null
}>()

const emit = defineEmits<{ close: [] }>()

const api = useApi()
const commentsStore = useCommentsStore()

// The menu's full-screen Teleport backdrop sits at <body>, so it is a sibling
// of the editor's scrollable container, not a descendant — wheel events over
// it never bubble into that container and the page appears frozen while the
// menu is open. Find the nearest scrollable ancestor of the editor's DOM and
// forward wheel events to it (scrollTarget=0 since the wheel already carries
// the current delta).
const editorEl = computed(() => props.editor?.view?.dom as HTMLElement | undefined)
function findScrollParent(el: HTMLElement | null | undefined): HTMLElement | undefined {
  let cur = el?.parentElement
  while (cur) {
    if (cur.scrollHeight > cur.clientHeight) return cur
    cur = cur.parentElement
  }
  return undefined
}
const scrollParent = computed(() => findScrollParent(editorEl.value))

// Clamp the menu inside the viewport once its real size is known. The initial
// y comes from coordsAtPos(selection.to).bottom, which lands near the bottom
// edge when the marked text runs off the screen — without this the menu opens
// half-invisible under the fold.
//
// offsetHeight is NOT reactive, so the height is tracked through a
// ResizeObserver instead: the menu grows when the AI panel (or comment draft)
// opens, and the clamp must re-evaluate with the taller size or the lower
// options end up below the viewport.
const menuEl = ref<HTMLElement | null>(null)
const menuHeight = ref(0)
let menuResizeObserver: ResizeObserver | undefined
onMounted(() => {
  const el = menuEl.value
  if (!el) return
  menuHeight.value = el.offsetHeight
  menuResizeObserver = new ResizeObserver(([entry]) => {
    menuHeight.value = entry.contentRect.height
  })
  menuResizeObserver.observe(el)
})
onBeforeUnmount(() => menuResizeObserver?.disconnect())

const clampedY = computed(() => {
  if (!menuHeight.value) return props.y
  const MENU_MARGIN = 8
  const menuH = menuHeight.value
  const viewportH = window.innerHeight
  if (props.y + menuH > viewportH - MENU_MARGIN) {
    return Math.max(MENU_MARGIN, props.y - menuH - 12)
  }
  return props.y
})
const clampedX = computed(() => {
  if (!menuEl.value) return props.x
  const MENU_MARGIN = 8
  const menuW = menuEl.value.offsetWidth
  if (props.x + menuW > window.innerWidth - MENU_MARGIN) {
    return Math.max(MENU_MARGIN, window.innerWidth - menuW - MENU_MARGIN)
  }
  return props.x
})

const textColors = [
  { name: 'Default', value: null },
  { name: 'Gray', value: '#787774' },
  { name: 'Brown', value: '#9F6B53' },
  { name: 'Orange', value: '#D9730D' },
  { name: 'Yellow', value: '#CB912F' },
  { name: 'Green', value: '#448361' },
  { name: 'Blue', value: '#337EA9' },
  { name: 'Purple', value: '#9065B0' },
  { name: 'Pink', value: '#C14C8A' },
  { name: 'Red', value: '#D44C47' },
]

const backgroundColors = [
  { name: 'Default', value: null },
  { name: 'Gray', value: '#F1F1EF' },
  { name: 'Brown', value: '#F4EEEE' },
  { name: 'Orange', value: '#FBECDD' },
  { name: 'Yellow', value: '#FBF3DB' },
  { name: 'Green', value: '#EDF3EC' },
  { name: 'Blue', value: '#E7F3F8' },
  { name: 'Purple', value: '#F6F3F9' },
  { name: 'Pink', value: '#FAF1F5' },
  { name: 'Red', value: '#FDEBEC' },
]

function blockRange() {
  return { from: props.pos, to: props.pos + props.node.nodeSize }
}

// Opened from a marked (dragged) text selection → format just that range.
// Opened via the grip/right-click with no selection → whole block, matching
// this menu's original block-scoped design.
function contentRange() {
  if (props.selectionRange) return props.selectionRange
  return { from: props.pos + 1, to: props.pos + props.node.nodeSize - 1 }
}

// Range covering every block a marked selection touches. blockRange() only
// covers the single hovered block under the pointer, so replacing a multi-line
// mark with it would rewrite just one line. $from.blockRange($to) resolves the
// deepest common block ancestor of the selection's endpoints — for a mark
// spanning several paragraphs that's the span from the first block's start to
// the last block's end (a selection ending exactly on a block boundary is
// handled natively, not inflated into the untouched next block); for a
// partial in-block mark it stays the marked text range, so a block-shaped AI
// reply splits that paragraph rather than nuking the whole line. Falls back to
// the hovered block when there's no selection.
function selectionBlockRange() {
  if (!props.selectionRange) return blockRange()
  const doc = props.editor.state.doc
  const r = doc
    .resolve(props.selectionRange.from)
    .blockRange(doc.resolve(props.selectionRange.to))
  return r ? { from: r.$from.pos, to: r.$to.pos } : blockRange()
}

function duplicate() {
  props.editor.chain().focus().insertContentAt(props.pos + props.node.nodeSize, props.node.toJSON()).run()
  emit('close')
}

function remove() {
  props.editor.chain().focus().deleteRange(blockRange()).run()
  emit('close')
}

function copyText() {
  navigator.clipboard.writeText(props.node.textContent)
  emit('close')
}

function setTextColor(value: string | null) {
  const chain = props.editor.chain().focus().setTextSelection(contentRange())
  if (value) chain.setColor(value).run()
  else chain.unsetColor().run()
  emit('close')
}

function setBackgroundColor(value: string | null) {
  const chain = props.editor.chain().focus().setTextSelection(contentRange())
  if (value) chain.setHighlight({ color: value }).run()
  else chain.unsetHighlight().run()
  emit('close')
}

// Formatting toggles act on the whole block's content, same as the color
// pickers above — this menu is block-scoped by design, not selection-scoped.
function toggleMark(mark: 'bold' | 'italic' | 'strike' | 'code') {
  const chain = props.editor.chain().focus().setTextSelection(contentRange())
  if (mark === 'bold') chain.toggleBold().run()
  else if (mark === 'italic') chain.toggleItalic().run()
  else if (mark === 'strike') chain.toggleStrike().run()
  else chain.toggleCode().run()
}

function isMarkActive(mark: string): boolean {
  return props.editor.isActive(mark)
}

// --- AI (BYOK) ---
// Same range this menu already uses for colors/formatting: the marked
// selection when opened by dragging over text, otherwise the whole hovered
// block — so "mark some text" and "click the line block button" both land
// here with the right scope for free.
const AI_ACTIONS = [
  { instruction: 'rephrase', label: 'Rephrase', hint: 'Reword, keeping its meaning' },
  { instruction: 'fix_grammar', label: 'Fix grammar', hint: 'Correct grammar and spelling' },
  { instruction: 'reformat', label: 'Reformat', hint: 'Restructure for clarity' },
  { instruction: 'proofread', label: 'Proofread', hint: 'Fix grammar, spelling and clarity' },
  { instruction: 'explain', label: 'Explain', hint: 'Add an explanation below' },
]
const aiPanelOpen = ref(false)
const aiLoading = ref(false)
const aiError = ref<{ message: string; settingsLink: boolean } | null>(null)
const aiStatus = useAiStatus()

function openAiPanel() {
  aiError.value = null
  aiPanelOpen.value = true
}

/** Renders the AI's markdown reply through the same converter the app's
 *  `.md` import uses, so `**bold**`, lists, code blocks, etc. come out as
 *  real editor nodes instead of literal syntax. AI prose uses the soft-break
 *  mode (`breaks: false`) so a single newline stays whitespace instead of
 *  becoming a hard <br> that re-serializes as `  \n`. */
function aiResultHtml(text: string): string {
  return markdownToHtml(text, { breaks: false }).trim()
}

// A rendered block element a text-selection replacement can't host inline.
const BLOCK_TAG = /^<(p|ul|ol|li|pre|blockquote|h[1-6]|hr|table)[\s>]/i

/** Replace (or append to) the current range with markdown-rendered AI output.
 *  Whole-block scoping replaces the full node so multi-block markdown renders
 *  as real blocks; a marked selection replaces just that text, inline — unless
 *  the result is block-shaped, in which case the whole selection's blocks are
 *  replaced so a list/heading/code reply doesn't get wedged into a paragraph
 *  and doesn't leave un-replaced lines from a multi-line mark. */
function insertAiResult(text: string, append: boolean) {
  const html = aiResultHtml(text)
  const inline = !!props.selectionRange && !BLOCK_TAG.test(html)
  const pos = append ? blockRange().to : inline ? contentRange() : selectionBlockRange()
  const content = inline ? html.replace(/^<p>([\s\S]*)<\/p>$/, '$1') : html
  props.editor.chain().focus().insertContentAt(pos, content).run()
}

type AiResult = { result: string; usage: { prompt: number; completion: number; total: number } | null }

// Shared error handling: surface in the popup and in the global toast so a
// dismissed menu doesn't swallow the failure.
function reportAiError(err: any) {
  const message = err?.data?.message
  const popup =
    message === 'no AI settings configured'
      ? { message: 'Set up your AI provider to use this.', settingsLink: true }
      : { message: message ?? 'The AI request failed.', settingsLink: false }
  aiError.value = popup
  aiStatus.fail(popup.message)
}

async function runAiAction(instruction: string) {
  const range = contentRange()
  const text = props.editor.state.doc.textBetween(range.from, range.to, '\n').trim()
  if (!text) {
    aiError.value = { message: 'There is no text here yet.', settingsLink: false }
    return
  }

  aiLoading.value = true
  aiError.value = null
  aiStatus.start('Running AI…')
  let data: AiResult
  try {
    data = await api<AiResult>('/ai/complete', { method: 'POST', body: { instruction, text } })
  } catch (err: any) {
    aiLoading.value = false
    reportAiError(err)
    return
  }
  aiLoading.value = false

  insertAiResult(data.result, instruction === 'explain')
  aiStatus.succeed('AI result applied', data.usage ?? null)
  emit('close')
}

// --- AI custom request (chat) ---
// The user's own prompt, optionally acting on the currently selected text —
// the selected text is appended as context so "make this more formal" works
// on a marked range without the user re-typing it. Replies append below the
// block, markdown-rendered, since a conversation shouldn't replace the
// selection it was prompted from.
const chatPrompt = ref('')

async function runAiChat() {
  const prompt = chatPrompt.value.trim()
  if (!prompt) return

  aiLoading.value = true
  aiError.value = null
  aiStatus.start('Running AI…')
  const range = contentRange()
  const selectedText = props.editor.state.doc.textBetween(range.from, range.to, '\n').trim()
  const text = selectedText ? `${prompt}\n\nSelected text:\n${selectedText}` : prompt
  let data: AiResult
  try {
    data = await api<AiResult>('/ai/complete', { method: 'POST', body: { instruction: 'chat', text } })
  } catch (err: any) {
    aiLoading.value = false
    reportAiError(err)
    return
  }
  aiLoading.value = false

  chatPrompt.value = ''
  insertAiResult(data.result, true)
  aiStatus.succeed('AI reply added', data.usage ?? null)
  emit('close')
}

// --- Comment ---
const commentDraftOpen = ref(false)
const commentBody = ref('')
const commentAssignee = ref('')
// People who can access this page (workspace members + page shares) — assignee
// dropdown options. Best-effort: some roles can't list every source.
const assigneeOptions = ref<{ email: string; label: string }[]>([])
const assigneesLoading = ref(false)

async function loadAssigneeOptions() {
  if (assigneeOptions.value.length || assigneesLoading.value) return
  assigneesLoading.value = true
  const byEmail = new Map<string, string>()

  try {
    const members = await api<{ user_id: string; email: string; role?: string }[]>(
      `/workspaces/${props.workspaceId}/members`,
    )
    for (const m of members) {
      if (m.email) byEmail.set(m.email.toLowerCase(), m.email)
    }
  } catch {
    // Non-member (e.g. page shared directly) can't list workspace members.
  }

  try {
    const grants = await api<
      { principal_type: string; email: string | null; pending?: boolean }[]
    >(`/pages/${props.pageId}/permissions`)
    for (const g of grants) {
      if (g.principal_type === 'user' && g.email && !g.pending) {
        byEmail.set(g.email.toLowerCase(), g.email)
      }
    }
  } catch {
    // Viewers can't list permissions — workspace members alone is fine.
  }

  assigneeOptions.value = Array.from(byEmail.values())
    .sort((a, b) => a.localeCompare(b))
    .map((email) => ({ email, label: email }))
  assigneesLoading.value = false
}

function openCommentDraft() {
  commentBody.value = ''
  commentAssignee.value = ''
  commentDraftOpen.value = true
  loadAssigneeOptions()
}

async function submitComment() {
  if (!commentBody.value.trim()) return
  const markId = crypto.randomUUID()
  props.editor.chain().focus().setTextSelection(contentRange()).setMark('comment', { commentId: markId }).run()
  await commentsStore.addComment(
    props.pageId,
    markId,
    commentBody.value.trim(),
    commentAssignee.value.trim() || undefined,
  )
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div class="fixed inset-0 z-40" @click="emit('close')" @contextmenu.prevent="emit('close')" @wheel.passive="scrollParent?.scrollBy(0, $event.deltaY)" />
    <div
      ref="menuEl"
      role="menu"
      class="fixed z-50 w-64 overflow-y-auto rounded-md border border-neutral-200 bg-white py-2 text-sm shadow-lg thin-scrollbar dark:border-neutral-700 dark:bg-neutral-900"
      :style="{ left: `${clampedX}px`, top: `${clampedY}px`, maxHeight: `calc(100vh - ${2 * 8}px)` }"
    >
      <template v-if="!commentDraftOpen && !aiPanelOpen">
        <div class="flex items-center gap-1 px-2 pb-2">
          <button
            type="button"
            title="Ask AI"
            aria-label="Ask AI"
            class="rounded p-1.5 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="openAiPanel"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <path d="m12 3 1.9 4.6L18.5 9.5l-4.6 1.9L12 16l-1.9-4.6L5.5 9.5l4.6-1.9L12 3Z" />
              <path d="M19 15v4M17 17h4" />
            </svg>
          </button>
          <button
            type="button"
            title="Duplicate"
            aria-label="Duplicate"
            class="rounded p-1.5 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="duplicate"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <rect x="9" y="9" width="13" height="13" rx="2" />
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
            </svg>
          </button>
          <button
            type="button"
            title="Copy text"
            aria-label="Copy text"
            class="rounded p-1.5 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="copyText"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
              <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
            </svg>
          </button>
          <button
            type="button"
            title="Delete"
            aria-label="Delete"
            class="rounded p-1.5 text-red-600 hover:bg-neutral-100 dark:text-red-400 dark:hover:bg-neutral-800"
            @click="remove"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6" />
            </svg>
          </button>
          <button
            v-if="pageId && workspaceId"
            type="button"
            title="Comment"
            aria-label="Comment"
            class="rounded p-1.5 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            @click="openCommentDraft"
          >
            💬
          </button>
        </div>

        <div class="my-1 border-t border-neutral-200 dark:border-neutral-800" />

        <div class="flex items-center gap-0.5 px-2 py-1.5">
          <button
            type="button"
            class="rounded px-2 py-1 text-sm font-bold text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            :class="isMarkActive('bold') ? 'bg-neutral-100 dark:bg-neutral-800' : ''"
            @click="toggleMark('bold')"
          >
            B
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 text-sm italic text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            :class="isMarkActive('italic') ? 'bg-neutral-100 dark:bg-neutral-800' : ''"
            @click="toggleMark('italic')"
          >
            I
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 text-sm line-through text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            :class="isMarkActive('strike') ? 'bg-neutral-100 dark:bg-neutral-800' : ''"
            @click="toggleMark('strike')"
          >
            S
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 font-mono text-sm text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
            :class="isMarkActive('code') ? 'bg-neutral-100 dark:bg-neutral-800' : ''"
            @click="toggleMark('code')"
          >
            &lt;/&gt;
          </button>
        </div>

        <div class="my-1 border-t border-neutral-200 dark:border-neutral-800" />

        <p class="px-3 pb-1 pt-1 text-xs font-medium text-neutral-400 dark:text-neutral-500">Text color</p>
        <div class="flex flex-wrap gap-1 px-3 pb-2">
          <button
            v-for="c in textColors"
            :key="c.name"
            type="button"
            :title="c.name"
            class="h-5 w-5 rounded border border-neutral-300 dark:border-neutral-600"
            :style="{ color: c.value ?? 'inherit', backgroundColor: 'transparent' }"
            @click="setTextColor(c.value)"
          >
            A
          </button>
        </div>

        <p class="px-3 pb-1 text-xs font-medium text-neutral-400 dark:text-neutral-500">Background</p>
        <div class="flex flex-wrap gap-1 px-3 pb-2">
          <button
            v-for="c in backgroundColors"
            :key="c.name"
            type="button"
            :title="c.name"
            class="h-5 w-5 rounded border border-neutral-300 dark:border-neutral-600"
            :style="{ backgroundColor: c.value ?? 'transparent' }"
            @click="setBackgroundColor(c.value)"
          />
        </div>
      </template>

      <div v-else-if="commentDraftOpen" class="px-2">
        <textarea
          v-model="commentBody"
          rows="3"
          placeholder="Add a comment…"
          autofocus
          class="w-full resize-none rounded border border-neutral-300 px-2 py-1.5 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
        />
        <label class="mt-2 block">
          <span class="mb-1 block text-[11px] font-medium text-neutral-500 dark:text-neutral-400">
            Assign to
          </span>
          <select
            v-model="commentAssignee"
            class="w-full rounded border border-neutral-300 bg-white px-2 py-1.5 text-xs text-neutral-900 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
          >
            <option value="">Unassigned</option>
            <option v-if="assigneesLoading" value="" disabled>Loading people…</option>
            <option
              v-for="opt in assigneeOptions"
              :key="opt.email"
              :value="opt.email"
            >
              {{ opt.label }}
            </option>
          </select>
          <p
            v-if="!assigneesLoading && assigneeOptions.length === 0"
            class="mt-1 text-[10px] text-neutral-400 dark:text-neutral-500"
          >
            No page collaborators to assign yet.
          </p>
        </label>
        <div class="mt-2 flex justify-end gap-2 pb-1">
          <button
            type="button"
            class="rounded px-2 py-1 text-xs font-medium text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
            @click="commentDraftOpen = false"
          >
            Cancel
          </button>
          <button
            type="button"
            :disabled="!commentBody.trim()"
            class="rounded bg-teal-600 px-2 py-1 text-xs font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
            @click="submitComment"
          >
            Comment
          </button>
        </div>
      </div>

      <div v-else class="px-2">
        <p class="px-1 pb-1.5 text-xs font-medium text-neutral-400 dark:text-neutral-500">
          Ask AI — uses your own provider (<NuxtLink to="/app/settings" class="underline" @click="emit('close')">settings</NuxtLink>)
        </p>

        <div v-if="aiLoading" class="px-1 py-3 text-sm text-neutral-500 dark:text-neutral-400">
          Thinking…
        </div>

        <template v-else>
          <p v-if="aiError" class="mb-2 rounded bg-red-50 px-2 py-1.5 text-xs text-red-700 dark:bg-red-950 dark:text-red-300">
            {{ aiError.message }}
            <NuxtLink v-if="aiError.settingsLink" to="/app/settings" class="underline" @click="emit('close')">
              Open settings
            </NuxtLink>
          </p>

          <div class="flex flex-col gap-1.5">
            <textarea
              v-model="chatPrompt"
              rows="2"
              placeholder="Ask anything…"
              class="w-full resize-none rounded border border-neutral-300 bg-white px-2 py-1.5 text-sm text-neutral-900 outline-none placeholder:text-neutral-400 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100 dark:placeholder:text-neutral-500"
              @keydown.enter.exact.prevent="runAiChat"
            />
            <button
              type="button"
              :disabled="!chatPrompt.trim() || aiLoading"
              class="self-end rounded bg-teal-600 px-2 py-1 text-xs font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
              @click="runAiChat"
            >
              Ask
            </button>
          </div>

          <div class="my-2 border-t border-neutral-200 dark:border-neutral-800" />

          <button
            v-for="action in AI_ACTIONS"
            :key="action.instruction"
            type="button"
            class="block w-full rounded px-2 py-1.5 text-left hover:bg-neutral-100 dark:hover:bg-neutral-800"
            @click="runAiAction(action.instruction)"
          >
            <span class="block text-neutral-900 dark:text-neutral-100">{{ action.label }}</span>
            <span class="block text-xs text-neutral-500 dark:text-neutral-400">{{ action.hint }}</span>
          </button>
        </template>

        <div class="mt-1 flex justify-end pb-1">
          <button
            type="button"
            class="rounded px-2 py-1 text-xs font-medium text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
            @click="aiPanelOpen = false"
          >
            Back
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
