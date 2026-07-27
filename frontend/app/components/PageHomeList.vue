<script setup lang="ts">
import { DEFAULT_PAGE_ICON, type PageNode } from '~/stores/pages'

const props = defineProps<{ workspaceId: string }>()

const pagesStore = usePagesStore()
const expanded = ref(new Set<string>())
const dragId = ref<string | null>(null)
type DropZone = 'before' | 'after' | null
const dropHint = ref<{ id: string; zone: DropZone } | null>(null)

/** Roots in sidebar/manual order (order_index), not updated_at — so DnD sticks. */
const sortedRoots = computed(() =>
  [...pagesStore.pageTree].sort(
    (a, b) =>
      a.order_index - b.order_index ||
      new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
  ),
)

function open(id: string) {
  pagesStore.activePageId = id
}

async function toggleExpand(node: PageNode) {
  if (expanded.value.has(node.id)) {
    expanded.value.delete(node.id)
    expanded.value = new Set(expanded.value)
    return
  }
  expanded.value.add(node.id)
  expanded.value = new Set(expanded.value)
  if (!node.childrenLoaded) {
    await pagesStore.fetchChildPages(node.id, { reset: true })
  }
}

function canExpand(node: PageNode) {
  return !!(node.has_children || node.children.length)
}

function relativeTime(iso: string): string {
  const secs = Math.round((Date.now() - new Date(iso).getTime()) / 1000)
  if (secs < 60) return 'just now'
  const mins = Math.round(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hours = Math.round(mins / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.round(hours / 24)
  return `${days}d ago`
}

function zoneFromEvent(e: DragEvent, el: HTMLElement): Exclude<DropZone, null> {
  const rect = el.getBoundingClientRect()
  const ratio = rect.height > 0 ? (e.clientY - rect.top) / rect.height : 0.5
  return ratio < 0.5 ? 'before' : 'after'
}

function onDragStart(e: DragEvent, node: PageNode) {
  dragId.value = node.id
  e.dataTransfer?.setData('text/plain', node.id)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}

function onDragOver(e: DragEvent, node: PageNode) {
  e.preventDefault()
  const el = e.currentTarget as HTMLElement
  dropHint.value = { id: node.id, zone: zoneFromEvent(e, el) }
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

function dropLabel(zone: DropZone): string {
  if (zone === 'before') return 'Move above'
  if (zone === 'after') return 'Move below'
  return ''
}

function onDragLeave(e: DragEvent, node: PageNode) {
  const related = e.relatedTarget as Node | null
  if (related && (e.currentTarget as HTMLElement).contains(related)) return
  if (dropHint.value?.id === node.id) dropHint.value = null
}

function onDragEnd() {
  dragId.value = null
  dropHint.value = null
}

async function onDropOn(e: DragEvent, target: PageNode) {
  e.preventDefault()
  const pageId = e.dataTransfer?.getData('text/plain') || dragId.value
  const zone = dropHint.value?.id === target.id ? dropHint.value.zone : 'after'
  dragId.value = null
  dropHint.value = null
  if (!pageId || pageId === target.id || !zone) return

  // Home list is roots only — never nest; always sibling reorder.
  const list = sortedRoots.value
  const without = list.filter((p) => p.id !== pageId)
  const targetIdx = without.findIndex((p) => p.id === target.id)
  if (targetIdx < 0) return
  const insertAt = zone === 'before' ? targetIdx : targetIdx + 1
  const next = [...without]
  const moved = list.find((p) => p.id === pageId)
  if (!moved) return
  next.splice(insertAt, 0, moved)

  await Promise.all(
    next.map((p, i) => pagesStore.updatePage(p.id, { orderIndex: i, parentPageId: null })),
  )
  await pagesStore.fetchPages(props.workspaceId)
}
</script>

<template>
  <div class="mx-auto w-full max-w-3xl">
    <div class="mb-6">
      <h1 class="text-2xl font-bold text-neutral-900 dark:text-neutral-100">Documents</h1>
      <p class="mt-1 text-sm text-neutral-500 dark:text-neutral-400">
        Parent pages. Drag above/below a row to reorder (does not nest).
      </p>
    </div>

    <p
      v-if="pagesStore.rootsLoading && pagesStore.pageTree.length === 0"
      class="text-sm text-neutral-500 dark:text-neutral-400"
    >
      Loading…
    </p>

    <p
      v-else-if="pagesStore.pageTree.length === 0"
      class="text-sm text-neutral-500 dark:text-neutral-400"
    >
      No pages yet. Create one from the sidebar.
    </p>

    <ul
      v-else
      class="divide-y divide-neutral-200 rounded-lg border border-neutral-200 dark:divide-neutral-800 dark:border-neutral-800"
    >
      <li
        v-for="node in sortedRoots"
        :key="node.id"
        class="bg-white dark:bg-neutral-900"
        :class="dragId === node.id ? 'opacity-50' : ''"
      >
        <div
          class="relative flex cursor-grab items-center gap-2 px-3 py-2.5 active:cursor-grabbing hover:bg-neutral-50 dark:hover:bg-neutral-800/60"
          draggable="true"
          @dragstart="onDragStart($event, node)"
          @dragend="onDragEnd"
          @dragover="onDragOver($event, node)"
          @dragleave="onDragLeave($event, node)"
          @drop="onDropOn($event, node)"
        >
          <span
            v-if="dropHint?.id === node.id && dropHint.zone === 'before'"
            class="pointer-events-none absolute inset-x-2 top-0 z-10 h-0.5 rounded bg-neutral-800 dark:bg-neutral-200"
          />
          <span
            v-if="dropHint?.id === node.id && dropHint.zone === 'after'"
            class="pointer-events-none absolute inset-x-2 bottom-0 z-10 h-0.5 rounded bg-neutral-800 dark:bg-neutral-200"
          />
          <span
            v-if="dropHint?.id === node.id && dropHint.zone"
            class="pointer-events-none absolute right-3 top-1/2 z-10 -translate-y-1/2 rounded bg-sky-600 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-white shadow dark:bg-sky-500"
          >
            {{ dropLabel(dropHint.zone) }}
          </span>

          <button
            v-if="canExpand(node)"
            type="button"
            class="shrink-0 cursor-pointer rounded p-1 text-neutral-400 hover:bg-neutral-100 dark:hover:bg-neutral-800"
            @click.stop="toggleExpand(node)"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="h-3.5 w-3.5 transition-transform"
              :class="expanded.has(node.id) ? 'rotate-90' : ''"
            >
              <path d="M9 6l6 6-6 6" />
            </svg>
          </button>
          <span v-else class="w-6 shrink-0" />

          <span
            class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
            role="link"
            @click="open(node.id)"
          >
            <span class="shrink-0 text-lg">{{ node.icon || DEFAULT_PAGE_ICON }}</span>
            <span class="truncate font-medium text-neutral-900 dark:text-neutral-100">
              {{ node.title || 'Untitled' }}
            </span>
            <span class="ml-auto shrink-0 text-xs text-neutral-400 dark:text-neutral-500">
              {{ relativeTime(node.updated_at) }}
            </span>
          </span>
        </div>

        <ul
          v-if="expanded.has(node.id)"
          class="border-t border-neutral-100 bg-neutral-50 dark:border-neutral-800 dark:bg-neutral-950/50"
        >
          <li
            v-if="node.childrenLoading && !node.children.length"
            class="px-10 py-2 text-xs text-neutral-400"
          >
            Loading…
          </li>
          <li
            v-for="child in [...node.children].sort(
              (a, b) => a.order_index - b.order_index || a.id.localeCompare(b.id),
            )"
            :key="child.id"
            class="flex cursor-pointer items-center gap-2 px-10 py-2 hover:bg-neutral-100 dark:hover:bg-neutral-900"
            @click="open(child.id)"
          >
            <span class="shrink-0">{{ child.icon || DEFAULT_PAGE_ICON }}</span>
            <span class="truncate text-sm text-neutral-800 dark:text-neutral-200">
              {{ child.title || 'Untitled' }}
            </span>
          </li>
          <li v-if="node.childrenCursor">
            <button
              type="button"
              class="w-full cursor-pointer px-10 py-2 text-left text-xs text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-900"
              :disabled="node.childrenLoading"
              @click="pagesStore.fetchChildPages(node.id)"
            >
              {{ node.childrenLoading ? 'Loading…' : 'Load more children' }}
            </button>
          </li>
          <li
            v-else-if="node.childrenLoaded && !node.children.length && !node.childrenLoading"
            class="px-10 py-2 text-xs text-neutral-400"
          >
            No child pages
          </li>
        </ul>
      </li>
    </ul>

    <button
      v-if="!pagesStore.rootsFullyLoaded"
      type="button"
      class="mt-4 w-full cursor-pointer rounded-md border border-neutral-200 py-2 text-sm font-medium text-neutral-600 hover:bg-neutral-50 dark:border-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-900"
      :disabled="pagesStore.rootsLoading"
      @click="pagesStore.loadMoreRoots(workspaceId)"
    >
      {{ pagesStore.rootsLoading ? 'Loading…' : 'Load more' }}
    </button>
  </div>
</template>
