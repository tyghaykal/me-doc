<script setup lang="ts">
import { VueFlow, useVueFlow, type Connection, type Edge, type Node } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import { adapterForSource } from '~/utils/diagram/adapters'
import type { EdgeKind, GraphModel, GraphNode, NodeShape } from '~/utils/diagram/adapters/types'

const props = defineProps<{ source: string }>()
const emit = defineEmits<{ 'update:source': [string] }>()

const { isDark } = useTheme()
const instanceId = `flow-${Math.random().toString(36).slice(2)}`
const { onConnect, addEdges } = useVueFlow(instanceId)

const nodes = ref<Node[]>([])
const edges = ref<Edge[]>([])
// Positions survive re-parses so a drag isn't lost when the graph changes.
// ponytail: positions are session-local (not persisted to a layout Y.Map yet);
// on reload the deterministic auto-layout is reused. Add a `layout` Y.Map to
// persist manual placement across sessions.
const positions = new Map<string, { x: number; y: number }>()

let applyingRemote = false
let seq = 0

function layout(model: GraphModel): Map<string, { x: number; y: number }> {
  const indeg = new Map<string, number>()
  model.nodes.forEach((n) => indeg.set(n.id, 0))
  model.edges.forEach((e) => indeg.set(e.target, (indeg.get(e.target) ?? 0) + 1))
  const adj = new Map<string, string[]>()
  model.nodes.forEach((n) => adj.set(n.id, []))
  model.edges.forEach((e) => adj.get(e.source)?.push(e.target))

  const layerOf = new Map<string, number>()
  let frontier = model.nodes.filter((n) => (indeg.get(n.id) ?? 0) === 0).map((n) => n.id)
  if (!frontier.length && model.nodes.length) frontier = [model.nodes[0]!.id]
  const seen = new Set<string>()
  let layer = 0
  while (frontier.length) {
    const next: string[] = []
    for (const id of frontier) {
      if (seen.has(id)) continue
      seen.add(id)
      layerOf.set(id, layer)
      for (const t of adj.get(id) ?? []) if (!seen.has(t)) next.push(t)
    }
    frontier = next
    layer++
  }
  model.nodes.forEach((n) => layerOf.has(n.id) || layerOf.set(n.id, 0))

  const horizontal = model.direction === 'LR' || model.direction === 'RL'
  const perLayer = new Map<number, number>()
  const out = new Map<string, { x: number; y: number }>()
  for (const n of model.nodes) {
    if (positions.has(n.id)) {
      out.set(n.id, positions.get(n.id)!)
      continue
    }
    const l = layerOf.get(n.id) ?? 0
    const idx = perLayer.get(l) ?? 0
    perLayer.set(l, idx + 1)
    out.set(n.id, horizontal ? { x: l * 220, y: idx * 100 } : { x: idx * 190, y: l * 120 })
  }
  return out
}

function shapeClass(shape: NodeShape): string {
  return `df-${shape}`
}

/** Rebuild the Vue Flow graph from Mermaid source (external/remote change). */
function rebuild(source: string) {
  const adapter = adapterForSource(source)
  const model = adapter?.parse(source)
  if (!model) return
  const pos = layout(model)
  nodes.value = model.nodes.map((n) => ({
    id: n.id,
    type: 'default',
    position: pos.get(n.id) ?? { x: 0, y: 0 },
    data: { label: n.label, shape: n.shape },
    class: shapeClass(n.shape),
  }))
  edges.value = model.edges.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    label: e.label,
    data: { kind: e.kind },
    animated: e.kind === 'dotted',
  }))
}

/** Serialize the current Vue Flow graph back to Mermaid and emit it. */
function regenerate() {
  const adapter = adapterForSource(props.source) ?? adapterForSource('graph TD')
  if (!adapter) return
  const direction = adapterForSource(props.source)?.parse(props.source)?.direction ?? 'TD'
  const model: GraphModel = {
    direction,
    nodes: nodes.value.map((n) => ({
      id: n.id,
      label: String(n.data?.label ?? n.id),
      shape: (n.data?.shape as NodeShape) ?? 'rect',
    })) as GraphNode[],
    edges: edges.value.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      label: typeof e.label === 'string' ? e.label : undefined,
      kind: (e.data?.kind as EdgeKind) ?? 'arrow',
    })),
  }
  applyingRemote = true
  emit('update:source', adapter.generate(model))
  // Release the guard after the prop round-trips back.
  nextTick(() => (applyingRemote = false))
}

onConnect((c: Connection) => {
  addEdges([{ id: `e${Date.now()}${seq++}`, source: c.source!, target: c.target!, data: { kind: 'arrow' } }])
  regenerate()
})

function onNodeDragStop(e: { node: Node }) {
  positions.set(e.node.id, { ...e.node.position })
}

function onNodesChange() {
  // Persist positions and pick up removals, then re-emit.
  nodes.value.forEach((n) => positions.set(n.id, { ...n.position }))
}

function addNode() {
  const id = nextNodeId()
  nodes.value = [
    ...nodes.value,
    {
      id,
      type: 'default',
      position: { x: 60, y: 60 },
      data: { label: id, shape: 'rect' },
      class: 'df-rect',
    },
  ]
  regenerate()
}

function nextNodeId(): string {
  const used = new Set(nodes.value.map((n) => n.id))
  for (let i = 0; i < 260; i++) {
    const id = String.fromCharCode(65 + (i % 26)) + (i >= 26 ? Math.floor(i / 26) : '')
    if (!used.has(id)) return id
  }
  return `n${Date.now()}`
}

function onNodeDoubleClick(e: { node: Node }) {
  const label = window.prompt('Node label', String(e.node.data?.label ?? ''))
  if (label == null) return
  const n = nodes.value.find((x) => x.id === e.node.id)
  if (n) n.data = { ...n.data, label }
  regenerate()
}

function onEdgeDoubleClick(e: { edge: Edge }) {
  const label = window.prompt('Edge label', typeof e.edge.label === 'string' ? e.edge.label : '')
  if (label == null) return
  const ed = edges.value.find((x) => x.id === e.edge.id)
  if (ed) ed.label = label
  regenerate()
}

// Keyboard delete of the selected node/edge.
function onKeydown(ev: KeyboardEvent) {
  if (ev.key !== 'Delete' && ev.key !== 'Backspace') return
  const selNodes = nodes.value.filter((n) => n.selected).map((n) => n.id)
  const selEdges = edges.value.filter((e) => e.selected).map((e) => e.id)
  if (!selNodes.length && !selEdges.length) return
  ev.preventDefault()
  if (selNodes.length) {
    const drop = new Set(selNodes)
    nodes.value = nodes.value.filter((n) => !drop.has(n.id))
    edges.value = edges.value.filter((e) => !drop.has(e.source) && !drop.has(e.target))
  }
  if (selEdges.length) {
    const drop = new Set(selEdges)
    edges.value = edges.value.filter((e) => !drop.has(e.id))
  }
  regenerate()
}

watch(
  () => props.source,
  (s) => {
    if (applyingRemote) return
    rebuild(s)
  },
  { immediate: true },
)
</script>

<template>
  <div
    class="diagram-flow relative h-full w-full bg-neutral-50 dark:bg-neutral-950"
    tabindex="0"
    @keydown="onKeydown"
  >
    <button
      type="button"
      class="absolute left-3 top-3 z-10 rounded-lg border border-neutral-200 bg-white/90 px-2.5 py-1 text-xs font-medium text-neutral-600 shadow-sm backdrop-blur hover:bg-neutral-100 dark:border-neutral-800 dark:bg-neutral-900/90 dark:text-neutral-300 dark:hover:bg-neutral-800"
      @click="addNode"
    >
      + Node
    </button>
    <div class="pointer-events-none absolute bottom-3 left-3 z-10 text-[11px] text-neutral-400 dark:text-neutral-600">
      Drag between handles to connect · double-click to rename · Del to remove
    </div>

    <VueFlow
      :id="instanceId"
      v-model:nodes="nodes"
      v-model:edges="edges"
      :class="isDark ? 'df-dark' : ''"
      fit-view-on-init
      @node-drag-stop="onNodeDragStop"
      @nodes-change="onNodesChange"
      @node-double-click="onNodeDoubleClick"
      @edge-double-click="onEdgeDoubleClick"
    >
      <Background :gap="16" :pattern-color="isDark ? '#3f3f46' : '#e5e5e5'" />
      <Controls />
    </VueFlow>
  </div>
</template>

<!-- Not scoped: Vue Flow's classes are global, and dark mode keys off the
     `.dark` class on <html>, an ancestor a scoped selector can't reach.
     Everything is namespaced under `.diagram-flow` to avoid leaking. -->
<style>
.diagram-flow .vue-flow__node-default {
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 500;
  border-radius: 8px;
  border: 1px solid #e5e5e5;
  background: #ffffff;
  color: #171717;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}
.diagram-flow .vue-flow__node-default.selected,
.diagram-flow .vue-flow__node-default:focus {
  border-color: #737373;
  box-shadow: 0 0 0 2px rgba(115, 115, 115, 0.35);
}
.diagram-flow .df-round,
.diagram-flow .df-stadium,
.diagram-flow .df-circle {
  border-radius: 9999px;
}
.diagram-flow .df-circle {
  aspect-ratio: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.diagram-flow .vue-flow__edge-path {
  stroke: #a3a3a3;
  stroke-width: 1.5;
}
.diagram-flow .vue-flow__edge.selected .vue-flow__edge-path {
  stroke: #525252;
}
.diagram-flow .vue-flow__edge-text {
  fill: #525252;
  font-size: 11px;
}
.diagram-flow .vue-flow__handle {
  background: #a3a3a3;
  border: none;
  width: 7px;
  height: 7px;
}
.diagram-flow .vue-flow__controls {
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
}
.diagram-flow .vue-flow__controls-button {
  background: #ffffff;
  border-bottom: 1px solid #e5e5e5;
  fill: #525252;
}
.diagram-flow .vue-flow__controls-button:hover {
  background: #f5f5f5;
}

/* Dark theme — matches the app's neutral-800/900 surfaces. */
.dark .diagram-flow .vue-flow__node-default {
  background: #262626;
  border-color: #404040;
  color: #e5e5e5;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.dark .diagram-flow .vue-flow__node-default.selected,
.dark .diagram-flow .vue-flow__node-default:focus {
  border-color: #a3a3a3;
  box-shadow: 0 0 0 2px rgba(163, 163, 163, 0.35);
}
.dark .diagram-flow .vue-flow__edge-path {
  stroke: #6b7280;
}
.dark .diagram-flow .vue-flow__edge.selected .vue-flow__edge-path {
  stroke: #d4d4d4;
}
.dark .diagram-flow .vue-flow__edge-text {
  fill: #d4d4d4;
}
.dark .diagram-flow .vue-flow__handle {
  background: #6b7280;
}
.dark .diagram-flow .vue-flow__controls-button {
  background: #262626;
  border-bottom-color: #404040;
  fill: #d4d4d4;
}
.dark .diagram-flow .vue-flow__controls-button:hover {
  background: #333333;
}
</style>
