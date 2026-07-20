<script setup lang="ts">
import type { PageNode } from '~/stores/pages'

const props = withDefaults(defineProps<{ nodes: PageNode[]; workspaceId: string; depth?: number }>(), {
  depth: 0,
})

const pagesStore = usePagesStore()

const contextMenu = ref<{ x: number; y: number; node: PageNode } | null>(null)

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
</script>

<template>
  <ul
    class="text-sm text-slate-700 dark:text-slate-300"
    :class="depth === 0 ? 'space-y-0.5' : ''"
    @dragover.prevent
    @drop="depth === 0 ? onDropRoot($event) : undefined"
  >
    <li v-if="depth === 0" class="mb-1">
      <button
        class="w-full rounded px-2 py-1 text-left text-xs font-medium text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-200"
        @click="addTopLevel"
      >
        + New page
      </button>
    </li>

    <li v-for="node in nodes" :key="node.id">
      <div
        draggable="true"
        class="group flex items-center gap-1 rounded px-2 py-1 hover:bg-slate-100 dark:hover:bg-slate-800"
        :class="pagesStore.activePageId === node.id ? 'bg-slate-100 font-medium text-slate-900 dark:bg-slate-800 dark:text-slate-100' : ''"
        :style="{ paddingLeft: `${depth * 12 + 8}px` }"
        @click="select(node)"
        @contextmenu.prevent="openContextMenu($event, node)"
        @dragstart="onDragStart($event, node)"
        @dragover.prevent
        @drop="onDropOnNode($event, node)"
      >
        <span class="flex-1 truncate">{{ node.title || 'Untitled' }}</span>
        <button
          class="opacity-0 group-hover:opacity-100 rounded px-1 text-slate-400 hover:bg-slate-200 hover:text-slate-700 dark:text-slate-500 dark:hover:bg-slate-700 dark:hover:text-slate-300"
          title="Add child page"
          @click.stop="addChild(node)"
        >
          +
        </button>
      </div>

      <PageTree
        v-if="node.children.length"
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
        class="fixed z-50 w-40 rounded-md border border-slate-200 bg-white py-1 text-sm shadow-lg dark:border-slate-700 dark:bg-slate-900"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      >
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-1.5 text-left text-slate-700 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-slate-800"
          @click="duplicateNode"
        >
          Duplicate
        </button>
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-1.5 text-left text-red-600 hover:bg-slate-50 dark:text-red-400 dark:hover:bg-slate-800"
          @click="deleteNode"
        >
          Delete
        </button>
      </div>
    </template>
  </Teleport>
</template>
