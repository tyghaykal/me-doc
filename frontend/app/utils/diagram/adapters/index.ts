import type { DiagramAdapter } from './types'
import { flowchartAdapter } from './flowchart'
import { detectDiagramType } from '~/utils/diagram/mermaid'

// Registry keyed by normalized Mermaid diagram type. Types without an adapter
// (sequence, gantt, pie, …) fall back to read-only preview in the canvas.
const ADAPTERS: Record<string, DiagramAdapter> = {
  flowchart: flowchartAdapter,
}

/** The adapter for the given Mermaid source, or null when none supports it. */
export function adapterForSource(source: string): DiagramAdapter | null {
  return ADAPTERS[detectDiagramType(source)] ?? null
}

export type { DiagramAdapter, GraphModel, GraphNode, GraphEdge } from './types'
