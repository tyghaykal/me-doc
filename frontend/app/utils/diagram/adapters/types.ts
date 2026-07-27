// Shared model the visual canvas edits. An adapter converts a Mermaid diagram
// family to/from this graph so the drag-drop canvas and the text stay in sync.

export type NodeShape = 'rect' | 'round' | 'stadium' | 'circle' | 'diamond'
export type EdgeKind = 'arrow' | 'open' | 'dotted' | 'thick'

export interface GraphNode {
  id: string
  label: string
  shape: NodeShape
}

export interface GraphEdge {
  id: string
  source: string
  target: string
  label?: string
  kind: EdgeKind
}

export interface GraphModel {
  direction: string
  nodes: GraphNode[]
  edges: GraphEdge[]
}

export interface DiagramAdapter {
  type: string
  /** Parse Mermaid source into the graph model, or null if unsupported. */
  parse(source: string): GraphModel | null
  /** Serialize the graph model back to Mermaid source. */
  generate(model: GraphModel): string
}
