<script setup lang="ts">
import {
  useCommentsStore,
  commentAssigneeLabel,
  commentAuthorLabel,
  type Comment,
} from '~/stores/comments'

const props = defineProps<{
  pageId: string
  open: boolean
  focusedMarkId?: string | null
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const commentsStore = useCommentsStore()
const loading = ref(false)
const replyDrafts = ref<Record<string, string>>({})
const replying = ref<Record<string, boolean>>({})
const cardRefs = ref<Record<string, HTMLElement | null>>({})

const WIDTH_KEY = 'me-doc.comments-sidebar-width'
const MIN_WIDTH = 320
const MAX_WIDTH = 640
const DEFAULT_WIDTH = 400

const width = ref(DEFAULT_WIDTH)
const resizing = ref(false)

function clampWidth(w: number) {
  return Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, Math.round(w)))
}

function loadWidth() {
  try {
    const raw = localStorage.getItem(WIDTH_KEY)
    if (raw) width.value = clampWidth(Number(raw))
  } catch {
    /* ignore */
  }
}

function persistWidth() {
  try {
    localStorage.setItem(WIDTH_KEY, String(width.value))
  } catch {
    /* ignore */
  }
}

function onResizePointerDown(e: PointerEvent) {
  e.preventDefault()
  resizing.value = true
  const startX = e.clientX
  const startW = width.value

  const onMove = (ev: PointerEvent) => {
    // Drag handle is on the left edge; moving left increases width.
    width.value = clampWidth(startW + (startX - ev.clientX))
  }
  const onUp = () => {
    resizing.value = false
    persistWidth()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
}

function relativeTime(iso: string): string {
  const secs = Math.round((Date.now() - new Date(iso).getTime()) / 1000)
  if (secs < 60) return 'just now'
  const mins = Math.round(secs / 60)
  if (mins < 60) return `${mins} minute${mins === 1 ? '' : 's'} ago`
  const hours = Math.round(mins / 60)
  if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`
  const days = Math.round(hours / 24)
  return `${days} day${days === 1 ? '' : 's'} ago`
}

async function load() {
  loading.value = true
  try {
    await commentsStore.fetchComments(props.pageId)
  } finally {
    loading.value = false
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      loadWidth()
      load()
    }
  },
)

watch(
  () => props.pageId,
  () => {
    if (props.open) load()
  },
)

function setCardRef(markId: string, el: unknown) {
  cardRefs.value[markId] = (el as HTMLElement | null) ?? null
}

function scrollToComment(markId: string) {
  document
    .querySelector(`[data-comment-id="${markId}"]`)
    ?.scrollIntoView({ behavior: 'smooth', block: 'center' })
}

function focusCard(markId: string) {
  nextTick(() => {
    cardRefs.value[markId]?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  })
}

watch(
  () => [props.open, props.focusedMarkId, loading.value] as const,
  ([isOpen, markId, isLoading]) => {
    if (isOpen && markId && !isLoading) focusCard(markId)
  },
)

async function toggleResolve(id: string) {
  await commentsStore.resolveComment(id)
}

async function remove(id: string) {
  await commentsStore.deleteComment(id)
}

async function submitReply(parent: Comment) {
  const body = (replyDrafts.value[parent.id] || '').trim()
  if (!body) return
  replying.value[parent.id] = true
  try {
    await commentsStore.addReply(props.pageId, parent.id, body)
    replyDrafts.value[parent.id] = ''
  } finally {
    replying.value[parent.id] = false
  }
}

function close() {
  emit('update:open', false)
}

// The Yjs "comment" mark has no resolved concept — only Postgres does — so
// keep the highlight's visual state in sync with the fetched list here.
watch(
  () => commentsStore.comments.map((c) => `${c.mark_id}:${c.resolved}:${c.parent_id ?? ''}`).join(','),
  () => {
    for (const c of commentsStore.roots()) {
      document.querySelectorAll(`[data-comment-id="${c.mark_id}"]`).forEach((el) => {
        el.classList.toggle('comment-resolved', c.resolved)
        el.classList.toggle(
          'comment-highlight-active',
          !!props.focusedMarkId && props.focusedMarkId === c.mark_id,
        )
      })
    }
  },
)

watch(
  () => props.focusedMarkId,
  (markId) => {
    document.querySelectorAll('.comment-highlight-active').forEach((el) => {
      el.classList.remove('comment-highlight-active')
    })
    if (markId) {
      document.querySelectorAll(`[data-comment-id="${markId}"]`).forEach((el) => {
        el.classList.add('comment-highlight-active')
      })
    }
  },
)
</script>

<template>
  <aside
    v-if="open"
    class="fixed right-0 top-0 z-40 flex h-screen shrink-0 flex-col border-l border-neutral-200 bg-white font-sans dark:border-neutral-800 dark:bg-neutral-900"
    :class="resizing ? 'select-none' : ''"
    :style="{ width: `${width}px` }"
  >
    <!-- Drag handle: pull left to widen, right to narrow -->
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize comments panel"
      title="Drag to resize"
      class="absolute inset-y-0 left-0 z-10 w-1.5 cursor-col-resize hover:bg-neutral-300/70 dark:hover:bg-neutral-600/70"
      :class="resizing ? 'bg-neutral-400/80 dark:bg-neutral-500/80' : ''"
      @pointerdown="onResizePointerDown"
    />

    <div class="flex shrink-0 items-center justify-between border-b border-neutral-200 px-4 py-3 dark:border-neutral-800">
      <h2 class="text-sm font-bold text-neutral-900 dark:text-neutral-100">Comments</h2>
      <button
        type="button"
        aria-label="Close"
        class="text-neutral-400 hover:text-neutral-600 dark:text-neutral-500 dark:hover:text-neutral-300"
        @click="close"
      >
        ✕
      </button>
    </div>

    <div class="thin-scrollbar min-h-0 flex-1 overflow-y-auto p-4">
      <p v-if="loading" class="text-sm text-neutral-400 dark:text-neutral-500">Loading…</p>
      <p
        v-else-if="commentsStore.roots().length === 0"
        class="text-sm text-neutral-400 dark:text-neutral-500"
      >
        No comments yet. Select some text and click 💬 to add one.
      </p>

      <ul v-else class="space-y-3">
        <li
          v-for="c in commentsStore.roots()"
          :key="c.id"
          :ref="(el) => setCardRef(c.mark_id, el)"
          class="rounded-lg border border-neutral-200 p-3 text-sm dark:border-neutral-800"
          :class="[
            c.resolved ? 'opacity-60' : '',
            focusedMarkId === c.mark_id
              ? 'border-amber-400 ring-2 ring-amber-300/60 dark:border-amber-500 dark:ring-amber-500/40'
              : '',
          ]"
        >
          <div class="cursor-pointer" @click="scrollToComment(c.mark_id)">
            <div class="flex items-center justify-between gap-2">
              <span
                class="truncate text-xs font-semibold text-neutral-500 dark:text-neutral-400"
                :title="c.author_email"
              >{{ commentAuthorLabel(c) }}</span>
              <span class="shrink-0 text-xs text-neutral-400 dark:text-neutral-500">{{
                relativeTime(c.created_at)
              }}</span>
            </div>
            <p class="mt-1.5 whitespace-pre-wrap text-[13px] leading-5 text-neutral-900 dark:text-neutral-100">{{ c.body }}</p>
            <div
              v-if="commentAssigneeLabel(c)"
              class="mt-2 flex items-center gap-1.5"
            >
              <span class="text-[11px] text-neutral-500 dark:text-neutral-400">Assigned to</span>
              <span
                class="inline-flex max-w-full items-center truncate rounded-full bg-sky-100 px-2 py-0.5 text-[11px] font-medium text-sky-800 dark:bg-sky-900/40 dark:text-sky-200"
                :title="c.assignee_email || undefined"
              >
                {{ commentAssigneeLabel(c) }}
              </span>
            </div>
            <p v-else class="mt-2 text-[11px] text-neutral-400 dark:text-neutral-500">Unassigned</p>
          </div>

          <div class="mt-2.5 flex items-center gap-2">
            <button
              type="button"
              class="text-xs font-medium text-neutral-500 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100"
              @click="toggleResolve(c.id)"
            >
              {{ c.resolved ? 'Reopen' : 'Resolve' }}
            </button>
            <button
              type="button"
              class="text-xs font-medium text-neutral-500 hover:text-red-600 dark:text-neutral-400 dark:hover:text-red-400"
              @click="remove(c.id)"
            >
              Delete
            </button>
          </div>

          <!-- Replies -->
          <ul
            v-if="commentsStore.repliesOf(c.id).length"
            class="mt-3 space-y-2 border-l-2 border-neutral-200 pl-3 dark:border-neutral-700"
          >
            <li
              v-for="r in commentsStore.repliesOf(c.id)"
              :key="r.id"
              class="rounded-md bg-neutral-50 p-2.5 dark:bg-neutral-800/50"
            >
              <div class="flex items-center justify-between gap-2">
                <span
                  class="truncate text-[11px] font-semibold text-neutral-500 dark:text-neutral-400"
                  :title="r.author_email"
                >{{ commentAuthorLabel(r) }}</span>
                <span class="shrink-0 text-[10px] text-neutral-400 dark:text-neutral-500">{{
                  relativeTime(r.created_at)
                }}</span>
              </div>
              <p class="mt-1 whitespace-pre-wrap text-xs leading-5 text-neutral-900 dark:text-neutral-100">
                {{ r.body }}
              </p>
              <button
                type="button"
                class="mt-1 text-[11px] font-medium text-neutral-500 hover:text-red-600 dark:text-neutral-400 dark:hover:text-red-400"
                @click="remove(r.id)"
              >
                Delete
              </button>
            </li>
          </ul>

          <!-- Reply composer -->
          <form class="mt-3 flex gap-1.5" @submit.prevent="submitReply(c)">
            <input
              v-model="replyDrafts[c.id]"
              type="text"
              placeholder="Reply…"
              class="min-w-0 flex-1 rounded border border-neutral-300 bg-white px-2.5 py-1.5 text-xs text-neutral-900 placeholder:text-neutral-400 dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100 dark:placeholder:text-neutral-500"
            />
            <button
              type="submit"
              :disabled="replying[c.id] || !(replyDrafts[c.id] || '').trim()"
              class="rounded bg-neutral-900 px-2.5 py-1.5 text-xs font-medium text-white disabled:opacity-40 dark:bg-neutral-100 dark:text-neutral-900"
            >
              {{ replying[c.id] ? '…' : 'Reply' }}
            </button>
          </form>
        </li>
      </ul>
    </div>
  </aside>
</template>
