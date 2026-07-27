// Thin wrapper around mermaid: lazy-loaded (client-only, code-split) so it never
// runs during SSR and doesn't bloat the initial bundle. Theme is passed per call
// and mermaid re-initialized, since the app theme can flip at runtime.

type MermaidModule = typeof import('mermaid')['default']

let modPromise: Promise<MermaidModule> | null = null
function load(): Promise<MermaidModule> {
  if (!modPromise) modPromise = import('mermaid').then((m) => m.default)
  return modPromise
}

// mermaid.render needs a unique DOM id per call.
let seq = 0
export function nextRenderId(): string {
  return `mmd-${Date.now().toString(36)}-${seq++}`
}

function init(mermaid: MermaidModule, dark: boolean) {
  mermaid.initialize({
    startOnLoad: false,
    theme: dark ? 'dark' : 'default',
    securityLevel: 'strict', // no raw HTML / script in labels
    fontFamily: 'inherit',
    flowchart: { htmlLabels: true },
  })
}

export interface RenderResult {
  svg?: string
  error?: string
}

/** Render Mermaid source to an SVG string, or return a parse/render error. */
export async function renderMermaid(source: string, dark: boolean): Promise<RenderResult> {
  const trimmed = source.trim()
  if (!trimmed) return { svg: '' }
  try {
    const mermaid = await load()
    init(mermaid, dark)
    const { svg } = await mermaid.render(nextRenderId(), trimmed)
    return { svg }
  } catch (e: unknown) {
    return { error: errorMessage(e) }
  }
}

/** Validate source; returns an error message, or null when it parses cleanly. */
export async function parseMermaid(source: string): Promise<string | null> {
  const trimmed = source.trim()
  if (!trimmed) return null
  try {
    const mermaid = await load()
    await mermaid.parse(trimmed)
    return null
  } catch (e: unknown) {
    return errorMessage(e)
  }
}

function errorMessage(e: unknown): string {
  if (e instanceof Error) return e.message
  if (typeof e === 'string') return e
  return 'Invalid diagram syntax'
}

/**
 * The mermaid diagram type declared by the source's first meaningful line,
 * normalized so callers/adapters can key on a stable name. Returns '' when
 * empty/unknown.
 */
export function detectDiagramType(source: string): string {
  const first =
    source
      .split('\n')
      .map((l) => l.trim())
      .find((l) => l && !l.startsWith('%%')) ?? ''
  const keyword = first.split(/[\s{:]/)[0]?.toLowerCase() ?? ''
  const map: Record<string, string> = {
    graph: 'flowchart',
    flowchart: 'flowchart',
    'flowchart-elk': 'flowchart',
    sequencediagram: 'sequence',
    classdiagram: 'class',
    'classdiagram-v2': 'class',
    statediagram: 'state',
    'statediagram-v2': 'state',
    erdiagram: 'er',
    gantt: 'gantt',
    pie: 'pie',
    journey: 'journey',
    gitgraph: 'gitgraph',
    mindmap: 'mindmap',
    timeline: 'timeline',
    quadrantchart: 'quadrant',
  }
  return map[keyword] ?? keyword
}
