<script setup lang="ts">
import { DEFAULT_PAGE_ICON, type PageNode } from '~/stores/pages'

const props = withDefaults(defineProps<{ nodes: PageNode[]; workspaceId: string; depth?: number }>(), {
  depth: 0,
})

const pagesStore = usePagesStore()

const contextMenu = ref<{ x: number; y: number; node: PageNode } | null>(null)

// Shared across every recursive instance of this component (same key = same state).
const collapsed = useState<Set<string>>('pageTree-collapsed', () => new Set())

function toggleCollapsed(node: PageNode) {
  if (collapsed.value.has(node.id)) collapsed.value.delete(node.id)
  else collapsed.value.add(node.id)
}

function select(node: PageNode) {
  pagesStore.activePageId = node.id
}

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

function onDragStart(e: DragEvent, node: PageNode) {
  e.dataTransfer?.setData('text/plain', node.id)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}

// Drop onto a node → that node becomes the new parent, appended to its children.
function onDropOnNode(e: DragEvent, target: PageNode) {
  e.stopPropagation()
  const pageId = e.dataTransfer?.getData('text/plain')
  if (!pageId || pageId === target.id) return
  pagesStore.updatePage(pageId, {
    parentPageId: target.id,
    orderIndex: target.children.length,
  })
}

// Drop on the root container → make top-level.
function onDropRoot(e: DragEvent) {
  const pageId = e.dataTransfer?.getData('text/plain')
  if (!pageId) return
  pagesStore.updatePage(pageId, {
    parentPageId: null,
    orderIndex: pagesStore.pageTree.length,
  })
}

function addChild(parent: PageNode) {
  const ws = parent.workspace_id
  if (ws) pagesStore.createPage(ws, { title: 'Untitled', parentPageId: parent.id })
}

function addTopLevel() {
  pagesStore.createPage(props.workspaceId, { title: 'Untitled' })
}

const importInput = ref<HTMLInputElement | null>(null)
function triggerImport() {
  importInput.value?.click()
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!)
}

async function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return

  const text = await file.text()
  const isMarkdown = file.name.toLowerCase().endsWith('.md')
  const html = isMarkdown
    ? markdownToHtml(text)
    : text
        .split(/\n{2,}/)
        .map((p) => `<p>${escapeHtml(p).replace(/\n/g, '<br>')}</p>`)
        .join('')

  const title = file.name.replace(/\.(md|txt)$/i, '')
  const page = await pagesStore.createPage(props.workspaceId, { title })
  pagesStore.setPendingImport(page.id, html)
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
        title="Import a .txt or .md file"
        class="rounded px-2 py-1 text-xs font-medium text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
        @click="triggerImport"
      >
        Import
      </button>
      <input ref="importInput" type="file" accept=".md,.txt" class="hidden" @change="onImportFile" />
    </li>

    <li v-for="node in nodes" :key="node.id">
      <div
        draggable="true"
        class="group flex items-center gap-1 rounded px-2 py-1 hover:bg-neutral-100 dark:hover:bg-neutral-800"
        :class="pagesStore.activePageId === node.id ? 'bg-neutral-100 font-medium text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100' : ''"
        :style="{ paddingLeft: `${depth * 12 + 8}px` }"
        @click="select(node)"
        @contextmenu.prevent="openContextMenu($event, node)"
        @dragstart="onDragStart($event, node)"
        @dragover.prevent
        @drop="onDropOnNode($event, node)"
      >
        <button
          v-if="node.children.length"
          class="shrink-0 rounded p-0.5 text-neutral-400 hover:bg-neutral-200 hover:text-neutral-700 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
          title="Toggle children"
          @click.stop="toggleCollapsed(node)"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3 w-3 transition-transform"
            :class="collapsed.has(node.id) ? '' : 'rotate-90'"
          >
            <path d="M9 6l6 6-6 6" />
          </svg>
        </button>
        <span v-else class="w-3.5 shrink-0" />
        <span class="shrink-0">{{ node.icon || DEFAULT_PAGE_ICON }}</span>
        <span class="flex-1 truncate">{{ node.title || 'Untitled' }}</span>
        <button
          class="opacity-0 group-hover:opacity-100 rounded px-1 text-neutral-400 hover:bg-neutral-200 hover:text-neutral-700 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
          title="Add child page"
          @click.stop="addChild(node)"
        >
          +
        </button>
      </div>

      <PageTree
        v-if="node.children.length && !collapsed.has(node.id)"
        :nodes="node.children"
        :workspace-id="workspaceId"
        :depth="depth + 1"
      />
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
