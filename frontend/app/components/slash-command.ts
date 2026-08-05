import { Extension } from '@tiptap/core'
import Suggestion from '@tiptap/suggestion'
import { VueRenderer } from '@tiptap/vue-3'
import SlashCommandList from './SlashCommandList.vue'
import { DEFAULT_DIAGRAM_SOURCE } from '~/utils/diagram/templates'

export interface SlashCommandItem {
  title: string
  description: string
  group: string
  icon: string
  keywords?: string
  command: (props: { editor: any; range: any }) => void
}

function buildCommands(opts: SlashCommandOptions): SlashCommandItem[] {
  const { onInsertImage, onEmbedDiagram } = opts
  return [
    {
      title: 'Text',
      description: 'Just start writing with plain text.',
      group: 'Basic',
      icon: 'T',
      keywords: 'paragraph plain',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).setParagraph().run(),
    },
    {
      title: 'Heading 1',
      description: 'Large section heading.',
      group: 'Basic',
      icon: 'H1',
      keywords: 'h1 title',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).setNode('heading', { level: 1 }).run(),
    },
    {
      title: 'Heading 2',
      description: 'Medium section heading.',
      group: 'Basic',
      icon: 'H2',
      keywords: 'h2 subtitle',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).setNode('heading', { level: 2 }).run(),
    },
    {
      title: 'Heading 3',
      description: 'Small section heading.',
      group: 'Basic',
      icon: 'H3',
      keywords: 'h3',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).setNode('heading', { level: 3 }).run(),
    },
    {
      title: 'Heading 4',
      description: 'Extra small section heading.',
      group: 'Basic',
      icon: 'H4',
      keywords: 'h4',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).setNode('heading', { level: 4 }).run(),
    },
    {
      title: 'Heading 5',
      description: 'Tiny section heading.',
      group: 'Basic',
      icon: 'H5',
      keywords: 'h5',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).setNode('heading', { level: 5 }).run(),
    },
    {
      title: 'Heading 6',
      description: 'Smallest section heading.',
      group: 'Basic',
      icon: 'H6',
      keywords: 'h6',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).setNode('heading', { level: 6 }).run(),
    },
    {
      title: 'To-do list',
      description: 'Track tasks with a checklist.',
      group: 'Lists',
      icon: '☐',
      keywords: 'todo task checkbox',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).toggleTaskList().run(),
    },
    {
      title: 'Bulleted list',
      description: 'Create a simple bulleted list.',
      group: 'Lists',
      icon: '•',
      keywords: 'ul unordered',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).toggleBulletList().run(),
    },
    {
      title: 'Numbered list',
      description: 'Create a list with numbering.',
      group: 'Lists',
      icon: '1.',
      keywords: 'ol ordered',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).toggleOrderedList().run(),
    },
    {
      title: 'Quote',
      description: 'Capture a quote or callout.',
      group: 'Blocks',
      icon: '❝',
      keywords: 'blockquote citation',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).toggleBlockquote().run(),
    },
    {
      title: 'Code block',
      description: 'Capture a code snippet.',
      group: 'Blocks',
      icon: '</>',
      keywords: 'code snippet pre',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).toggleCodeBlock().run(),
    },
    {
      title: 'Divider',
      description: 'Visually divide blocks.',
      group: 'Blocks',
      icon: '—',
      keywords: 'hr horizontal rule separator',
      command: ({ editor, range }) => editor.chain().focus().deleteRange(range).setHorizontalRule().run(),
    },
    {
      title: 'Image',
      description: 'Upload or embed an image.',
      group: 'Media',
      icon: '▣',
      keywords: 'picture photo media',
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).run()
        onInsertImage()
      },
    },
    {
      title: 'Table',
      description: 'Insert a simple table.',
      group: 'Blocks',
      icon: '▦',
      keywords: 'table grid rows columns',
      command: ({ editor, range }) =>
        editor.chain().focus().deleteRange(range).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
    },
    {
      title: 'Diagram',
      description: 'Draw a Mermaid chart or diagram.',
      group: 'Media',
      icon: '◈',
      keywords: 'diagram chart mermaid flowchart graph sequence',
      command: ({ editor, range }) =>
        editor
          .chain()
          .focus()
          .deleteRange(range)
          .insertContent({ type: 'diagram', attrs: { source: DEFAULT_DIAGRAM_SOURCE } })
          .run(),
    },
    // Only meaningful with a server workspace to pick an existing diagram from.
    ...(onEmbedDiagram
      ? [
          {
            title: 'Embed diagram',
            description: 'Insert a live copy of an existing diagram.',
            group: 'Media',
            icon: '⧉',
            keywords: 'embed diagram chart link reference',
            command: ({ editor, range }: { editor: any; range: any }) => {
              editor.chain().focus().deleteRange(range).run()
              onEmbedDiagram()
            },
          },
        ]
      : []),
  ]
}

/** Place the menu under the caret when there's room; flip above when near the bottom. */
function positionPopup(popup: HTMLElement, clientRect: (() => DOMRect | null) | null | undefined) {
  const rect = clientRect?.()
  if (!rect) return

  const menu = popup.firstElementChild as HTMLElement | null
  const menuHeight = menu?.offsetHeight ?? 320
  const menuWidth = menu?.offsetWidth ?? 320
  const gap = 6
  const pad = 8

  const spaceBelow = window.innerHeight - rect.bottom
  const placeAbove = spaceBelow < menuHeight + gap && rect.top > spaceBelow

  let top = placeAbove ? rect.top - menuHeight - gap : rect.bottom + gap
  let left = rect.left

  // Keep inside the viewport horizontally.
  left = Math.max(pad, Math.min(left, window.innerWidth - menuWidth - pad))
  // Keep inside the viewport vertically (scrollable menu handles overflow).
  top = Math.max(pad, Math.min(top, window.innerHeight - menuHeight - pad))

  popup.style.left = `${left}px`
  popup.style.top = `${top}px`
}

export interface SlashCommandOptions {
  onInsertImage: () => void
  onEmbedDiagram?: () => void
}

export const SlashCommand = Extension.create<SlashCommandOptions>({
  name: 'slashCommand',

  addOptions() {
    return {
      onInsertImage: () => {},
      onEmbedDiagram: undefined,
    }
  },

  addProseMirrorPlugins() {
    const commands = buildCommands(this.options)

    return [
      Suggestion({
        editor: this.editor,
        char: '/',
        startOfLine: false,
        items: ({ query }: { query: string }) => {
          const q = query.toLowerCase().trim()
          if (!q) return commands
          return commands.filter((item) => {
            const hay = `${item.title} ${item.description} ${item.keywords ?? ''} ${item.group}`.toLowerCase()
            return hay.includes(q)
          })
        },
        command: ({ editor, range, props }: any) => {
          props.command({ editor, range })
        },
        render: () => {
          let component: VueRenderer
          let popup: HTMLElement

          return {
            onStart: (props: any) => {
              component = new VueRenderer(SlashCommandList, {
                props: {
                  items: props.items,
                  command: (item: SlashCommandItem) => item.command({ editor: props.editor, range: props.range }),
                },
                editor: props.editor,
              })

              popup = document.createElement('div')
              popup.style.position = 'fixed'
              popup.style.zIndex = '60'
              popup.style.top = '0'
              popup.style.left = '0'
              document.body.appendChild(popup)
              popup.appendChild(component.element as HTMLElement)
              // Measure after mount so flip uses real height.
              requestAnimationFrame(() => positionPopup(popup, props.clientRect))
            },
            onUpdate: (props: any) => {
              component.updateProps({
                items: props.items,
                command: (item: SlashCommandItem) => item.command({ editor: props.editor, range: props.range }),
              })
              requestAnimationFrame(() => positionPopup(popup, props.clientRect))
            },
            onKeyDown: (props: any) => {
              if (props.event.key === 'Escape') {
                popup.remove()
                return true
              }
              return (component.ref as any)?.onKeyDown(props) ?? false
            },
            onExit: () => {
              popup.remove()
              component.destroy()
            },
          }
        },
      }),
    ]
  },
})
