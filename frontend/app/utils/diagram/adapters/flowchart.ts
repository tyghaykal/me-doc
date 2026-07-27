import type { DiagramAdapter, EdgeKind, GraphModel, GraphNode, NodeShape } from './types'

// Parser/generator for the common Mermaid flowchart subset:
//   graph TD / flowchart LR
//   A[rect]  B(round)  C([stadium])  D((circle))  E{diamond}
//   A --> B    A --- B    A -.-> B    A ==> B    A -->|label| B
// Anything it can't parse confidently → returns null so the canvas falls back
// to read-only preview (never a broken editor).
// ponytail: single edge per line, no subgraphs/styling — extend as needed.

const HEADER_RE = /^\s*(?:graph|flowchart)\s+([A-Za-z]{2})\b/i
const EDGE_OPS: Record<string, EdgeKind> = {
  '-.->': 'dotted',
  '-->': 'arrow',
  '==>': 'thick',
  '---': 'open',
}
// Longest-first so '-.->' wins over '-->' etc.
const OP_RE = /(-\.->|-->|==>|---)/

interface ParsedToken {
  id: string
  label?: string
  shape?: NodeShape
}

// Extract a node's id + optional shape/label from the start of a chunk.
function parseToken(chunk: string): ParsedToken | null {
  const m = chunk
    .trim()
    .match(/^([A-Za-z0-9_]+)\s*(\(\(([^)]*)\)\)|\(\[([^\]]*)\]\)|\[([^\]]*)\]|\(([^)]*)\)|\{([^}]*)\})?/)
  if (!m) return null
  const id = m[1]!
  if (m[3] !== undefined) return { id, shape: 'circle', label: m[3] }
  if (m[4] !== undefined) return { id, shape: 'stadium', label: m[4] }
  if (m[5] !== undefined) return { id, shape: 'rect', label: m[5] }
  if (m[6] !== undefined) return { id, shape: 'round', label: m[6] }
  if (m[7] !== undefined) return { id, shape: 'diamond', label: m[7] }
  return { id }
}

// Constructs this simple adapter can't faithfully regenerate. If present, we
// decline (return null) so the visual canvas falls back to a read-only preview
// rather than silently dropping subgraphs/styling on the next edit.
const UNSUPPORTED_LINE = /^(subgraph|style|classDef|class|linkStyle|click)\b/i
function hasUnsupported(source: string): boolean {
  if (source.includes(':::')) return true
  return source.split('\n').some((l) => UNSUPPORTED_LINE.test(l.trim()))
}

function parse(source: string): GraphModel | null {
  if (hasUnsupported(source)) return null
  const lines = source.split('\n')
  const header = lines.find((l) => HEADER_RE.test(l))
  if (!header) return null
  const direction = (header.match(HEADER_RE)![1] || 'TD').toUpperCase()

  const nodes = new Map<string, GraphNode>()
  const edges: GraphModel['edges'] = []
  const ensure = (t: ParsedToken) => {
    const existing = nodes.get(t.id)
    if (!existing) {
      nodes.set(t.id, { id: t.id, label: t.label ?? t.id, shape: t.shape ?? 'rect' })
    } else if (t.label !== undefined) {
      existing.label = t.label
      if (t.shape) existing.shape = t.shape
    }
  }

  let edgeSeq = 0
  for (const raw of lines) {
    const line = raw.trim()
    if (!line || HEADER_RE.test(line) || line.startsWith('%%')) continue

    const op = line.match(OP_RE)
    if (!op) {
      const t = parseToken(line)
      if (t) ensure(t)
      continue
    }

    const idx = op.index!
    const left = line.slice(0, idx)
    let rest = line.slice(idx + op[0].length)
    let label: string | undefined
    const labelMatch = rest.match(/^\s*\|([^|]*)\|/)
    if (labelMatch) {
      label = labelMatch[1]!.trim()
      rest = rest.slice(labelMatch[0].length)
    }
    const from = parseToken(left)
    const to = parseToken(rest)
    if (!from || !to) return null
    ensure(from)
    ensure(to)
    edges.push({
      id: `e${edgeSeq++}`,
      source: from.id,
      target: to.id,
      label: label || undefined,
      kind: EDGE_OPS[op[1]!] ?? 'arrow',
    })
  }

  if (nodes.size === 0) return null
  return { direction, nodes: [...nodes.values()], edges }
}

function wrap(shape: NodeShape, label: string): string {
  switch (shape) {
    case 'round':
      return `(${label})`
    case 'stadium':
      return `([${label}])`
    case 'circle':
      return `((${label}))`
    case 'diamond':
      return `{${label}}`
    default:
      return `[${label}]`
  }
}

function opFor(kind: EdgeKind): string {
  return (Object.keys(EDGE_OPS) as string[]).find((k) => EDGE_OPS[k] === kind) ?? '-->'
}

function generate(model: GraphModel): string {
  const out: string[] = [`graph ${model.direction || 'TD'}`]
  for (const n of model.nodes) out.push(`  ${n.id}${wrap(n.shape, n.label)}`)
  for (const e of model.edges) {
    const lbl = e.label ? `|${e.label}|` : ''
    out.push(`  ${e.source} ${opFor(e.kind)}${lbl} ${e.target}`)
  }
  return out.join('\n') + '\n'
}

export const flowchartAdapter: DiagramAdapter = { type: 'flowchart', parse, generate }
