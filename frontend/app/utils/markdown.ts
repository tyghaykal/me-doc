import { Marked } from 'marked'
import markedFootnote from 'marked-footnote'

// A dedicated instance rather than the package's shared default `marked`
// singleton: `.use()` always appends (never replaces/dedupes) tokenizers
// onto whatever instance it's called on. The shared singleton persists
// across Vite HMR reloads of *this* file (only this module re-evaluates,
// not the `marked` package itself), so every edit during dev was silently
// re-running `.use()` on the same long-lived object and piling up
// duplicate/stale registrations. A fresh instance per module evaluation
// sidesteps that entirely.
const marked = new Marked()

marked.setOptions({ breaks: true })
marked.use(markedFootnote())

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function escapeHtmlText(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

// `*[KEY]: full title` lines define an abbreviation — stripped from the text
// here and applied as whole-word substitutions in walkTokens below. Reset
// per conversion so definitions from a previous call never leak into the next.
let abbreviations = new Map<string, string>()

function extractAbbreviations(text: string): string {
  abbreviations = new Map()
  return text.replace(/^\*\[([^\]]+)\]:[ \t]*(.+)$/gm, (_m, key: string, title: string) => {
    abbreviations.set(key, title.trim())
    return ''
  })
}

function isDefMarkerLine(line: string): boolean {
  return /^[ \t]{0,3}[:~][ \t]+\S/.test(line)
}

function isBlank(line: string): boolean {
  return /^\s*$/.test(line)
}

// Common markdown-it-style shorthand `marked` doesn't parse out of the box.
// Most map onto marks/nodes our editor schema already understands.
marked.use({
  extensions: [
    {
      name: 'mark',
      level: 'inline',
      start(src: string) {
        return src.match(/==(?!=)/)?.index
      },
      tokenizer(this: any, src: string) {
        const match = /^==(?!=)([^=]+?)==(?!=)/.exec(src)
        if (!match) return undefined
        return { type: 'mark', raw: match[0], tokens: this.lexer.inlineTokens(match[1]) }
      },
      renderer(this: any, token: any) {
        return `<mark>${this.parser.parseInline(token.tokens)}</mark>`
      },
    },
    {
      name: 'ins',
      level: 'inline',
      start(src: string) {
        return src.match(/\+\+(?!\+)/)?.index
      },
      tokenizer(this: any, src: string) {
        const match = /^\+\+(?!\+)([^+]+?)\+\+(?!\+)/.exec(src)
        if (!match) return undefined
        return { type: 'ins', raw: match[0], tokens: this.lexer.inlineTokens(match[1]) }
      },
      renderer(this: any, token: any) {
        return `<u>${this.parser.parseInline(token.tokens)}</u>`
      },
    },
    {
      name: 'superscript',
      level: 'inline',
      start(src: string) {
        return src.match(/\^/)?.index
      },
      tokenizer(this: any, src: string) {
        const match = /^\^([^^\n]+?)\^/.exec(src)
        if (!match) return undefined
        return { type: 'superscript', raw: match[0], tokens: this.lexer.inlineTokens(match[1]) }
      },
      renderer(this: any, token: any) {
        return `<sup>${this.parser.parseInline(token.tokens)}</sup>`
      },
    },
    {
      name: 'subscript',
      level: 'inline',
      start(src: string) {
        return src.match(/~(?!~)/)?.index
      },
      tokenizer(this: any, src: string) {
        const match = /^~(?!~)([^~\n]+?)~(?!~)/.exec(src)
        if (!match) return undefined
        return { type: 'subscript', raw: match[0], tokens: this.lexer.inlineTokens(match[1]) }
      },
      renderer(this: any, token: any) {
        return `<sub>${this.parser.parseInline(token.tokens)}</sub>`
      },
    },
    {
      // `Term\n\n:   Definition` (or compact `  ~ Definition`, no blank line).
      // Not a byte-for-byte port of markdown-it-deflist (skips its nested
      // code-block-inside-a-definition case) but covers the common shapes.
      name: 'definitionList',
      level: 'block',
      start(src: string) {
        return src.match(/^[ \t]{0,3}[:~][ \t]+\S/m)?.index
      },
      tokenizer(this: any, src: string) {
        const lines = src.split('\n')
        let i = 0
        const termLines: string[] = []
        while (i < lines.length && !isBlank(lines[i]) && !isDefMarkerLine(lines[i])) {
          termLines.push(lines[i])
          i++
        }
        if (termLines.length === 0) return undefined

        let j = i
        if (j < lines.length && isBlank(lines[j])) j++
        if (j >= lines.length || !isDefMarkerLine(lines[j])) return undefined

        const definitions: string[] = []
        let k = j
        while (k < lines.length && isDefMarkerLine(lines[k])) {
          let def = lines[k].replace(/^[ \t]{0,3}[:~][ \t]+/, '')
          k++
          while (k < lines.length && !isBlank(lines[k]) && !isDefMarkerLine(lines[k])) {
            def += ' ' + lines[k].trim()
            k++
          }
          definitions.push(def.trim())
          if (k < lines.length && isBlank(lines[k])) {
            if (k + 1 < lines.length && isDefMarkerLine(lines[k + 1])) {
              k++
              continue
            }
            break
          }
        }
        if (definitions.length === 0) return undefined

        return {
          type: 'definitionList',
          raw: lines.slice(0, k).join('\n'),
          termTokens: termLines.map((t) => this.lexer.inlineTokens(t.trim())),
          defTokens: definitions.map((d) => this.lexer.inlineTokens(d)),
        }
      },
      renderer(this: any, token: any) {
        const dts = token.termTokens.map((t: any) => `<dt>${this.parser.parseInline(t)}</dt>`).join('')
        const dds = token.defTokens.map((d: any) => `<dd>${this.parser.parseInline(d)}</dd>`).join('')
        return `<dl>${dts}${dds}</dl>\n`
      },
    },
    {
      // ::: warning\n...content...\n:::
      name: 'container',
      level: 'block',
      start(src: string) {
        return src.match(/^ {0,3}:::/m)?.index
      },
      tokenizer(this: any, src: string) {
        const match = /^ {0,3}:::[ \t]*(\S*)[ \t]*\n([\s\S]*?)\n {0,3}:::[ \t]*(?:\n|$)/.exec(src)
        if (!match) return undefined
        return {
          type: 'container',
          raw: match[0],
          containerType: match[1] || 'info',
          tokens: this.lexer.blockTokens(match[2], []),
        }
      },
      renderer(this: any, token: any) {
        return `<div data-container="${token.containerType}">\n${this.parser.parse(token.tokens)}</div>\n`
      },
    },
  ],
  // Unambiguous typographic substitutions, plus whole-word abbreviation
  // wrapping — both only touch plain 'text' tokens, so code/codespan
  // content is never rewritten. Abbreviation matches switch the token to
  // marked's 'html' type (verbatim passthrough) since the mark itself needs
  // real <abbr> markup, not escaped text.
  walkTokens(token: any) {
    if (token.type !== 'text' || typeof token.text !== 'string') return

    token.text = token.text
      .replace(/\(c\)/gi, '©')
      .replace(/\(r\)/gi, '®')
      .replace(/\(tm\)/gi, '™')
      .replace(/\+-/g, '±')
      .replace(/\.\.\./g, '…')
      .replace(/---/g, '—')
      .replace(/--/g, '–')

    if (abbreviations.size === 0) return
    const keys = [...abbreviations.keys()].sort((a, b) => b.length - a.length).map(escapeRegExp)
    const re = new RegExp(`\\b(${keys.join('|')})\\b`, 'g')
    if (!re.test(token.text)) return

    re.lastIndex = 0
    let html = ''
    let last = 0
    let m: RegExpExecArray | null
    while ((m = re.exec(token.text))) {
      html += escapeHtmlText(token.text.slice(last, m.index))
      const title = abbreviations.get(m[1])!
      html += `<abbr title="${escapeHtmlText(title)}">${escapeHtmlText(m[1])}</abbr>`
      last = m.index + m[0].length
    }
    html += escapeHtmlText(token.text.slice(last))
    token.type = 'html'
    token.text = html
    token.raw = html
  },
})

export function markdownToHtml(text: string): string {
  return marked.parse(extractAbbreviations(text), { async: false })
}
