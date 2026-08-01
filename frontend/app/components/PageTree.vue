<script setup lang="ts">
import { DEFAULT_PAGE_ICON, type PageNode } from '~/stores/pages'

const props = withDefaults(defineProps<{ nodes: PageNode[]; workspaceId: string; depth?: number }>(), {
  depth: 0,
})

const pagesStore = usePagesStore()

const contextMenu = ref<{ x: number; y: number; node: PageNode } | null>(null)

// Shared across every recursive instance of this component (same key = same state).
// Ids in this set are collapsed. Parents with unloaded children default to collapsed.
const collapsed = useState<Set<string>>('pageTree-collapsed', () => new Set())

function isCollapsed(node: PageNode): boolean {
  if (collapsed.value.has(node.id)) return true
  if (expandedExplicit.value.has(node.id)) return false
  // Default: parents with unloaded children start collapsed.
  if (node.has_children && !node.childrenLoaded) return true
  return false
}

function canExpand(node: PageNode): boolean {
  // Once children are actually loaded, trust that over has_children — the
  // backend flag can go stale after the last child is dragged elsewhere.
  if (node.childrenLoaded) return node.children.length > 0
  return !!node.has_children
}

async function toggleCollapsed(node: PageNode) {
  if (isCollapsed(node)) {
    // Expand: remove from collapsed set and load children if needed.
    collapsed.value.delete(node.id)
    // Mark as explicitly expanded by ensuring it's not in the set; for first
    // expand of has_children nodes we also need a sentinel so isCollapsed
    // doesn't keep treating them as collapsed after load.
    collapsed.value = new Set(collapsed.value)
    // Track expanded parents that were never in the set: use a parallel set.
    expandedExplicit.value.add(node.id)
    expandedExplicit.value = new Set(expandedExplicit.value)
    if (!node.childrenLoaded) {
      await pagesStore.fetchChildPages(node.id, { reset: true })
    }
  } else {
    collapsed.value.add(node.id)
    collapsed.value = new Set(collapsed.value)
    expandedExplicit.value.delete(node.id)
    expandedExplicit.value = new Set(expandedExplicit.value)
  }
}

const expandedExplicit = useState<Set<string>>('pageTree-expanded', () => new Set())

function select(node: PageNode) {
  pagesStore.activePageId = node.id
  // PageTree is also rendered from pages that aren't `/app/[[pageId]]`
  // (e.g. the changelog) — those hosts have no route<->activePageId watcher,
  // so setting the store alone silently does nothing there. Navigate
  // directly instead of relying on a side effect that only exists on one host.
  navigateTo(`/app/${node.id}`)
}

// Keep the open page visible in the tree: explicitly expand its ancestor chain
// whenever it changes. Without this, a parent stays open only via the fragile
// "children happen to be loaded" default, so navigating to a sub-page (or its
// parent then back) could let the parent silently collapse and hide the child.
async function revealActive(id: string | null) {
  if (props.depth !== 0 || !id) return
  const byId = new Map(pagesStore.pages.map((p) => [p.id, p]))
  const ancestors: string[] = []
  const seen = new Set<string>()
  let cur = byId.get(id)?.parent_page_id ?? null
  while (cur && !seen.has(cur)) {
    seen.add(cur)
    ancestors.push(cur)
    cur = byId.get(cur)?.parent_page_id ?? null
  }
  if (!ancestors.length) return

  let changed = false
  for (const pid of ancestors) {
    if (collapsed.value.has(pid)) {
      collapsed.value.delete(pid)
      changed = true
    }
    if (!expandedExplicit.value.has(pid)) {
      expandedExplicit.value.add(pid)
      changed = true
    }
    // Load the branch so the active page's node actually exists under it.
    if (!pagesStore.childrenLoadedParents.has(pid)) {
      await pagesStore.fetchChildPages(pid, { reset: true })
    }
  }
  if (changed) {
    collapsed.value = new Set(collapsed.value)
    expandedExplicit.value = new Set(expandedExplicit.value)
  }
}

// Re-run when the open page changes and when pages first load (direct-URL open
// sets the active id before the tree is fetched).
watch(
  () => [pagesStore.activePageId, pagesStore.pages.length] as const,
  () => revealActive(pagesStore.activePageId),
  { immediate: true },
)

function openContextMenu(e: MouseEvent, node: PageNode) {
  contextMenu.value = { x: e.clientX, y: e.clientY, node }
}

function duplicateNode() {
  const node = contextMenu.value?.node
  contextMenu.value = null
  if (node) pagesStore.duplicatePage(node.id)
}

function deleteNode() {
  const node = contextMenu.value?.node
  contextMenu.value = null
  if (node && window.confirm(`Delete "${node.title || 'Untitled'}"? This can be restored from trash later.`)) {
    pagesStore.deletePage(node.id)
  }
}

// Drop zone while dragging over a row: top/bottom = sibling reorder, middle = nest.
type DropZone = 'before' | 'after' | 'into' | null
const dropHint = ref<{ id: string; zone: DropZone } | null>(null)
// Last non-null hint — dragleave often fires before drop and would wipe zone.
const lastDropHint = ref<{ id: string; zone: Exclude<DropZone, null> } | null>(null)
const draggingId = ref<string | null>(null)

function zoneFromEvent(e: DragEvent, el: HTMLElement): Exclude<DropZone, null> {
  const rect = el.getBoundingClientRect()
  const y = e.clientY - rect.top
  const ratio = rect.height > 0 ? y / rect.height : 0.5
  // Wider middle band so "Make child" is easier to hit.
  if (ratio < 0.22) return 'before'
  if (ratio > 0.78) return 'after'
  return 'into'
}

// A small custom drag preview instead of the browser's default full-row
// ghost — the default ghost is exactly as wide as the row, so it always
// covers the "Make child" pill on whatever row is underneath the cursor.
// This pill-sized one only covers a small area near the cursor, and still
// makes the drag visually obvious (unlike hiding the ghost entirely).
function buildDragGhost(node: PageNode): HTMLElement {
  const ghost = document.createElement('div')
  ghost.textContent = `${node.icon || DEFAULT_PAGE_ICON} ${node.title || 'Untitled'}`
  ghost.className =
    'fixed -left-96 top-0 flex items-center gap-1 rounded border border-neutral-300 bg-white px-2 py-1 text-xs text-neutral-700 shadow-md dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-200'
  document.body.appendChild(ghost)
  return ghost
}

function onDragStart(e: DragEvent, node: PageNode) {
  draggingId.value = node.id
  e.dataTransfer?.setData('text/plain', node.id)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
  const ghost = buildDragGhost(node)
  e.dataTransfer?.setDragImage(ghost, 12, 12)
  // The browser snapshots the ghost synchronously when setDragImage runs,
  // so it's safe to remove right after — it never needs to stay visible.
  setTimeout(() => ghost.remove(), 0)
}

function onDragOverRow(e: DragEvent, target: PageNode) {
  e.preventDefault()
  e.stopPropagation()
  if (draggingId.value === target.id) return
  const el = e.currentTarget as HTMLElement
  const zone = zoneFromEvent(e, el)
  dropHint.value = { id: target.id, zone }
  lastDropHint.value = { id: target.id, zone }
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

function onDragLeaveRow(e: DragEvent, target: PageNode) {
  const related = e.relatedTarget as Node | null
  if (related && (e.currentTarget as HTMLElement).contains(related)) return
  // Clear UI highlight only; keep lastDropHint for the actual drop event.
  if (dropHint.value?.id === target.id) dropHint.value = null
}

function wouldCreateCycle(pageId: string, newParentId: string): boolean {
  // Walking up from newParent, if we hit pageId then nesting would cycle.
  let cur: string | null | undefined = newParentId
  const byId = new Map(pagesStore.pages.map((p) => [p.id, p]))
  const seen = new Set<string>()
  while (cur) {
    if (cur === pageId) return true
    if (seen.has(cur)) break
    seen.add(cur)
    cur = byId.get(cur)?.parent_page_id ?? null
  }
  return false
}

async function nestAsChild(pageId: string, parent: PageNode) {
  if (wouldCreateCycle(pageId, parent.id)) return
  await pagesStore.updatePage(pageId, {
    parentPageId: parent.id,
    orderIndex: parent.children.length,
  })
  // Show the child under the parent immediately.
  expandedExplicit.value.add(parent.id)
  expandedExplicit.value = new Set(expandedExplicit.value)
  collapsed.value.delete(parent.id)
  collapsed.value = new Set(collapsed.value)
  await pagesStore.fetchChildPages(parent.id, { reset: true })
  // has_children is refreshed via fetchChildPages + mergePages
}

async function onDropOnNode(e: DragEvent, target: PageNode) {
  e.preventDefault()
  e.stopPropagation()
  const pageId = e.dataTransfer?.getData('text/plain') || draggingId.value
  const hint =
    (dropHint.value?.id === target.id && dropHint.value.zone
      ? dropHint.value
      : null) ||
    (lastDropHint.value?.id === target.id ? lastDropHint.value : null)
  const zone = hint?.zone ?? 'after'
  dropHint.value = null
  lastDropHint.value = null
  draggingId.value = null
  if (!pageId || pageId === target.id) return

  if (zone === 'into') {
    await nestAsChild(pageId, target)
    return
  }

  // Sibling reorder: same parent as target, order before/after target.
  const parentId = target.parent_page_id
  const siblings = parentId
    ? pagesStore.pages
        .filter((p) => p.parent_page_id === parentId)
        .sort((a, b) => a.order_index - b.order_index || a.id.localeCompare(b.id))
    : pagesStore.pages
        .filter((p) => !p.parent_page_id)
        .sort((a, b) => a.order_index - b.order_index || a.id.localeCompare(b.id))

  const without = siblings.filter((p) => p.id !== pageId)
  const targetIdx = without.findIndex((p) => p.id === target.id)
  if (targetIdx < 0) return
  const insertAt = zone === 'before' ? targetIdx : targetIdx + 1
  const orderedIds = without.map((p) => p.id)
  orderedIds.splice(insertAt, 0, pageId)

  await Promise.all(
    orderedIds.map((id, i) =>
      pagesStore.updatePage(id, {
        parentPageId: parentId ?? null,
        orderIndex: i,
      }),
    ),
  )
}

function onDropRoot(e: DragEvent) {
  e.preventDefault()
  const pageId = e.dataTransfer?.getData('text/plain') || draggingId.value
  dropHint.value = null
  lastDropHint.value = null
  draggingId.value = null
  if (!pageId) return
  pagesStore.updatePage(pageId, {
    parentPageId: null,
    orderIndex: pagesStore.pageTree.length,
  })
}

function onDragEnd() {
  dropHint.value = null
  lastDropHint.value = null
  draggingId.value = null
}

function addChild(parent: PageNode) {
  const ws = parent.workspace_id
  if (ws) pagesStore.createPage(ws, { title: 'Untitled', parentPageId: parent.id })
}

function addTopLevel() {
  pagesStore.createPage(props.workspaceId, { title: 'Untitled' })
}

function addDiagram() {
  pagesStore.createPage(props.workspaceId, { title: 'Untitled diagram', kind: 'diagram' })
}

const api = useApi()

const importInput = ref<HTMLInputElement | null>(null)
const importing = ref(false)
const importError = ref<string | null>(null)
function triggerImport() {
  importError.value = null
  importInput.value?.click()
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!)
}

// Anything MarkItDown can turn into Markdown — .txt/.md stay local (no
// network round-trip), everything else goes through the converter service.
const CONVERTIBLE_EXTENSIONS = /\.(docx|doc|pdf|xlsx|xls|pptx|ppt|epub|html?|csv)$/i

async function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return

  const name = file.name.toLowerCase()
  let html: string

  if (name.endsWith('.md') || name.endsWith('.txt')) {
    const text = await file.text()
    html = name.endsWith('.md')
      ? markdownToHtml(text)
      : text
          .split(/\n{2,}/)
          .map((p) => `<p>${escapeHtml(p).replace(/\n/g, '<br>')}</p>`)
          .join('')
  } else if (CONVERTIBLE_EXTENSIONS.test(name)) {
    importing.value = true
    importError.value = null
    try {
      const form = new FormData()
      form.append('file', file)
      const { markdown } = await api<{ markdown: string }>('/pages/import', { method: 'POST', body: form })
      html = markdownToHtml(markdown)
    } catch (err: any) {
      importError.value = err?.data?.message ?? err?.message ?? 'Import failed.'
      return
    } finally {
      importing.value = false
    }
  } else {
    importError.value = 'Unsupported file type.'
    return
  }

  const title = file.name.replace(/\.[^.]+$/, '')
  const page = await pagesStore.createPage(props.workspaceId, { title })
  pagesStore.setPendingImport(page.id, html)
}

async function loadMoreRoots() {
  await pagesStore.loadMoreRoots(props.workspaceId)
}

async function loadMoreChildren(parent: PageNode) {
  await pagesStore.fetchChildPages(parent.id)
}
</script>

<template>
  <ul
    class="text-sm text-neutral-700 dark:text-neutral-300"
    :class="depth === 0 ? 'space-y-0.5' : ''"
    @dragover.prevent
    @drop="depth === 0 ? onDropRoot($event) : undefined"
  >
    <li v-if="depth === 0" class="mb-1 flex items-center gap-1">
      <button
        class="flex-1 rounded px-2 py-1 text-left text-xs font-medium text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
        @click="addTopLevel"
      >
        + New page
      </button>
      <button
        type="button"
        title="New diagram"
        class="rounded px-2 py-1 text-xs font-medium text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
        @click="addDiagram"
      >
        + Diagram
      </button>
      <button
        type="button"
        :disabled="importing"
        title="Import a document (.txt, .md, .docx, .pdf, .xlsx, .pptx, .epub, ...)"
        class="rounded px-2 py-1 text-xs font-medium text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 disabled:opacity-50 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
        @click="triggerImport"
      >
        {{ importing ? 'Importing…' : 'Import' }}
      </button>
      <input
        ref="importInput"
        type="file"
        accept=".md,.txt,.docx,.doc,.pdf,.xlsx,.xls,.pptx,.ppt,.epub,.html,.htm,.csv"
        class="hidden"
        @change="onImportFile"
      />
    </li>
    <li v-if="depth === 0 && importError" class="mb-1 px-2 text-xs text-red-600 dark:text-red-400">
      {{ importError }}
    </li>

    <li v-for="node in nodes" :key="node.id">
      <div
        draggable="true"
        class="group relative flex cursor-grab items-center gap-1 rounded px-2 py-1.5 active:cursor-grabbing"
        :class="[
          dropHint?.id === node.id && dropHint.zone === 'into'
            ? 'bg-sky-50 ring-1 ring-inset ring-sky-400 dark:bg-sky-950/40 dark:ring-sky-500'
            : pagesStore.activePageId === node.id
              ? 'bg-teal-50 font-medium text-teal-900 hover:bg-teal-100 dark:bg-teal-950/40 dark:text-teal-200 dark:hover:bg-teal-950/60'
              : 'hover:bg-neutral-100 dark:hover:bg-neutral-800',
          draggingId === node.id ? 'opacity-50' : '',
        ]"
        :style="{ paddingLeft: `${depth * 12 + 8}px` }"
        @click="select(node)"
        @contextmenu.prevent="openContextMenu($event, node)"
        @dragstart="onDragStart($event, node)"
        @dragend="onDragEnd"
        @dragover="onDragOverRow($event, node)"
        @dragleave="onDragLeaveRow($event, node)"
        @drop="onDropOnNode($event, node)"
      >
        <!-- Sibling reorder indicators (top/bottom of row) -->
        <span
          v-if="dropHint?.id === node.id && dropHint.zone === 'before'"
          class="pointer-events-none absolute inset-x-1 top-0 z-10 h-0.5 rounded bg-neutral-800 dark:bg-neutral-200"
        />
        <span
          v-if="dropHint?.id === node.id && dropHint.zone === 'after'"
          class="pointer-events-none absolute inset-x-1 bottom-0 z-10 h-0.5 rounded bg-neutral-800 dark:bg-neutral-200"
        />
        <button
          v-if="canExpand(node)"
          class="shrink-0 cursor-pointer rounded p-0.5 text-neutral-400 hover:bg-neutral-200 hover:text-neutral-700 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
          title="Toggle children"
          @click.stop="toggleCollapsed(node)"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3 w-3 transition-transform"
            :class="isCollapsed(node) ? '' : 'rotate-90'"
          >
            <path d="M9 6l6 6-6 6" />
          </svg>
        </button>
        <span v-else class="w-3.5 shrink-0" />
        <span class="shrink-0">{{ node.icon || DEFAULT_PAGE_ICON }}</span>
        <!-- Title shrinks when nest label is shown so they never overlap -->
        <span
          class="min-w-0 flex-1 truncate"
          :class="dropHint?.id === node.id && dropHint.zone === 'into' ? 'pr-1' : ''"
        >{{ node.title || 'Untitled' }}</span>
        <span
          v-if="dropHint?.id === node.id && dropHint.zone === 'into'"
          class="pointer-events-none shrink-0 rounded bg-sky-600 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-white dark:bg-sky-500"
        >
          Make child
        </span>
        <button
          v-else
          class="cursor-pointer rounded px-1 text-neutral-400 opacity-0 hover:bg-neutral-200 hover:text-neutral-700 group-hover:opacity-100 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
          title="Add child page"
          @click.stop="addChild(node)"
        >
          +
        </button>
      </div>

      <template v-if="!isCollapsed(node) && canExpand(node)">
        <p
          v-if="node.childrenLoading && !node.children.length"
          class="px-2 py-1 text-xs text-neutral-400 dark:text-neutral-500"
          :style="{ paddingLeft: `${(depth + 1) * 12 + 8}px` }"
        >
          Loading…
        </p>
        <PageTree
          v-if="node.children.length"
          :nodes="node.children"
          :workspace-id="workspaceId"
          :depth="depth + 1"
        />
        <button
          v-if="node.childrenCursor"
          type="button"
          class="w-full rounded px-2 py-1 text-left text-xs text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
          :style="{ paddingLeft: `${(depth + 1) * 12 + 8}px` }"
          :disabled="node.childrenLoading"
          @click.stop="loadMoreChildren(node)"
        >
          {{ node.childrenLoading ? 'Loading…' : 'Load more' }}
        </button>
      </template>
    </li>

    <li v-if="depth === 0 && !pagesStore.rootsFullyLoaded">
      <button
        type="button"
        class="w-full rounded px-2 py-1.5 text-left text-xs font-medium text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
        :disabled="pagesStore.rootsLoading"
        @click="loadMoreRoots"
      >
        {{ pagesStore.rootsLoading ? 'Loading…' : 'Load more pages' }}
      </button>
    </li>
  </ul>

  <Teleport to="body">
    <template v-if="contextMenu">
      <div class="fixed inset-0 z-40" @click="contextMenu = null" @contextmenu.prevent="contextMenu = null" />
      <div
        role="menu"
        class="fixed z-50 w-40 rounded-md border border-neutral-200 bg-white py-1 text-sm shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      >
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-1.5 text-left text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="duplicateNode"
        >
          Duplicate
        </button>
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-1.5 text-left text-red-600 hover:bg-neutral-50 dark:text-red-400 dark:hover:bg-neutral-800"
          @click="deleteNode"
        >
          Delete
        </button>
      </div>
    </template>
  </Teleport>
</template>
