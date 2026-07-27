<script setup lang="ts">
import { useCommentsStore } from '~/stores/comments'

const props = defineProps<{
  editor: any
  pos: number
  node: any
  x: number
  y: number
  pageId: string
  workspaceId: string
  selectionRange?: { from: number; to: number } | null
}>()

const emit = defineEmits<{ close: [] }>()

const api = useApi()
const commentsStore = useCommentsStore()

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
    <div class="fixed inset-0 z-40" @click="emit('close')" @contextmenu.prevent="emit('close')" />
    <div
      role="menu"
      class="fixed z-50 w-64 rounded-md border border-neutral-200 bg-white py-2 text-sm shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
      :style="{ left: `${x}px`, top: `${y}px` }"
    >
      <template v-if="!commentDraftOpen">
        <div class="flex items-center gap-1 px-2 pb-2">
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

      <div v-else class="px-2">
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
    </div>
  </Teleport>
</template>
