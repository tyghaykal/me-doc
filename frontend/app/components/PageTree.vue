<script setup lang="ts">
import type { PageNode } from '~/stores/pages'

const props = withDefaults(defineProps<{ nodes: PageNode[]; depth?: number }>(), {
  depth: 0,
})

const pagesStore = usePagesStore()

function workspaceId() {
  return props.nodes[0]?.workspace_id ?? pagesStore.pages[0]?.workspace_id
}

function select(node: PageNode) {
  pagesStore.activePageId = node.id
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
  const ws = workspaceId()
  if (ws) pagesStore.createPage(ws, { title: 'Untitled' })
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

      <PageTree v-if="node.children.length" :nodes="node.children" :depth="depth + 1" />
    </li>
  </ul>
</template>
