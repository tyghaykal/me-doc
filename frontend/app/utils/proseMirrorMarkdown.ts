// Mirrors backend/src/export/mod.rs's `yjs_to_markdown` block-for-block, so a
// document saved locally as Markdown reads identically to one exported from a
// server-backed page (same heading/list/table/code-fence conventions). The
// backend walks a Yjs XML fragment; this walks the same node/mark shape as
// Tiptap's own `editor.getJSON()` — y-prosemirror maps ProseMirror node/mark
// type names onto Yjs XML tags 1:1, so the two walkers stay in lockstep by
// switching on the same names.

interface PMMark {
  type: string
  attrs?: Record<string, unknown>
}

interface PMNode {
  type: string
  attrs?: Record<string, unknown>
  content?: PMNode[]
  text?: string
  marks?: PMMark[]
}

export function editorJsonToMarkdown(doc: PMNode): string {
  const out: string[] = []
  for (const node of doc.content ?? []) renderBlock(node, out)
  const trimmed = out.join('').trimEnd()
  return trimmed ? `${trimmed}\n` : ''
}

function renderBlock(node: PMNode, out: string[]): void {
  switch (node.type) {
    case 'paragraph':
      out.push(`${renderInline(node.content ?? []).trimEnd()}\n\n`)
      break
    case 'heading': {
      const level = Math.min(6, Math.max(1, Number(node.attrs?.level) || 1))
      out.push(`${'#'.repeat(level)} ${renderInline(node.content ?? []).trimEnd()}\n\n`)
      break
    }
    case 'bulletList':
      out.push(renderList(node, 'bullet'), '\n')
      break
    case 'orderedList':
      out.push(renderList(node, 'ordered'), '\n')
      break
    case 'taskList':
      out.push(renderList(node, 'task'), '\n')
      break
    case 'table':
      out.push(renderTable(node))
      break
    case 'codeBlock': {
      const lang = (node.attrs?.language as string) || ''
      const code = (node.content ?? []).map((c) => c.text ?? '').join('')
      out.push('```', lang, '\n', code.replace(/\n+$/, ''), '\n```\n\n')
      break
    }
    case 'blockquote': {
      const inner: string[] = []
      for (const child of node.content ?? []) renderBlock(child, inner)
      const text = inner.join('').trimEnd()
      for (const line of text.split('\n')) out.push('> ', line, '\n')
      out.push('\n')
      break
    }
    case 'horizontalRule':
      out.push('---\n\n')
      break
    case 'image':
      out.push(renderImage(node), '\n\n')
      break
    // Inline diagram block: Mermaid source lives in the `source` attribute.
    case 'diagram': {
      const source = (node.attrs?.source as string) || ''
      out.push('```mermaid\n', source.replace(/\n+$/, ''), '\n```\n\n')
      break
    }
    // Live embed of a standalone diagram page — same placeholder link the
    // backend export emits (it also has no way to inline another page's
    // source without a server round-trip). Local mode has no server
    // workspace to embed from, so this is dead code today, kept only so a
    // pasted/imported diagramEmbed node doesn't silently vanish.
    case 'diagramEmbed':
      out.push(`[Embedded diagram](/app/${node.attrs?.diagramId ?? ''})\n\n`)
      break
    // Unknown block: recurse so nested text is never silently dropped.
    default:
      for (const child of node.content ?? []) renderBlock(child, out)
  }
}

function renderList(list: PMNode, kind: 'bullet' | 'ordered' | 'task'): string {
  let index = 1
  const lines: string[] = []
  for (const item of list.content ?? []) {
    let marker: string
    if (kind === 'ordered') {
      marker = `${index}. `
      index += 1
    } else if (kind === 'task') {
      marker = item.attrs?.checked ? '- [x] ' : '- [ ] '
    } else {
      marker = '- '
    }

    const itemOut: string[] = []
    for (const block of item.content ?? []) renderBlock(block, itemOut)
    const text = itemOut.join('').trimEnd()
    const itemLines = text ? text.split('\n') : ['']

    lines.push(marker + itemLines[0])
    for (const line of itemLines.slice(1)) lines.push(`  ${line}`)
  }
  return lines.map((l) => `${l}\n`).join('')
}

// GFM pipe table — Tiptap's table model always starts with a header row, so
// the separator row goes right after the first row unconditionally.
// Multi-paragraph cell content is flattened onto one line.
function renderTable(table: PMNode): string {
  const rows: string[][] = []
  for (const row of table.content ?? []) {
    if (row.type !== 'tableRow') continue
    const cells: string[] = []
    for (const cell of row.content ?? []) {
      if (cell.type !== 'tableCell' && cell.type !== 'tableHeader') continue
      const cellOut: string[] = []
      for (const block of cell.content ?? []) renderBlock(block, cellOut)
      cells.push(cellOut.join('').trim().replace(/\n/g, ' ').replace(/\|/g, '\\|'))
    }
    rows.push(cells)
  }

  const colCount = rows.reduce((m, r) => Math.max(m, r.length), 0)
  if (rows.length === 0 || colCount === 0) return ''

  let out = ''
  rows.forEach((cells, i) => {
    out += '|'
    for (let c = 0; c < colCount; c++) out += ` ${cells[c] ?? ''} |`
    out += '\n'
    if (i === 0) {
      out += '|';
      for (let c = 0; c < colCount; c++) out += ' --- |'
      out += '\n'
    }
  })
  return `${out}\n`
}

function renderInline(nodes: PMNode[]): string {
  let out = ''
  for (const n of nodes) {
    if (n.type === 'text') out += renderText(n)
    else if (n.type === 'hardBreak') out += '  \n'
    else if (n.type === 'image') out += renderImage(n)
    else out += renderInline(n.content ?? [])
  }
  return out
}

function renderImage(n: PMNode): string {
  return `![${(n.attrs?.alt as string) || ''}](${(n.attrs?.src as string) || ''})`
}

// Wraps a text run in Markdown marks, same order as the backend's `render_text`
// (code innermost, then bold/italic/strike, highlight and text-color as
// literal inline HTML since GFM has no standard syntax for either).
function renderText(n: PMNode): string {
  let text = n.text ?? ''
  const marks = n.marks ?? []
  const has = (type: string) => marks.some((m) => m.type === type)
  const colorOf = (type: string): string | undefined => {
    const c = marks.find((m) => m.type === type)?.attrs?.color
    return typeof c === 'string' && c ? c : undefined
  }

  if (has('code')) text = `\`${text}\``
  if (has('bold')) text = `**${text}**`
  if (has('italic')) text = `*${text}*`
  if (has('strike')) text = `~~${text}~~`
  if (has('highlight')) text = `<mark style="background:${colorOf('highlight') ?? '#fef08a'}">${text}</mark>`
  const textColor = colorOf('textStyle')
  if (textColor) text = `<span style="color:${textColor}">${text}</span>`
  return text
}
