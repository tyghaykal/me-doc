import { Node, mergeAttributes } from '@tiptap/core'
import { VueNodeViewRenderer } from '@tiptap/vue-3'
import DiagramEmbedView from './DiagramEmbedView.vue'

// A live, read-only embed of a standalone diagram page. Stores only the
// diagram's page id (+ a cached title for display/export); the NodeView mirrors
// that page's current Mermaid source over the shared collab room, so edits to
// the source diagram propagate to every document embedding it.
export const DiagramEmbed = Node.create({
  name: 'diagramEmbed',
  group: 'block',
  atom: true,
  draggable: true,
  selectable: true,

  addAttributes() {
    return {
      diagramId: { default: '' },
      title: { default: '' },
    }
  },

  parseHTML() {
    return [{ tag: 'div[data-diagram-embed]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(HTMLAttributes, {
        'data-diagram-embed': '',
        'data-diagram-id': HTMLAttributes.diagramId,
      }),
    ]
  },

  addNodeView() {
    return VueNodeViewRenderer(DiagramEmbedView)
  },
})
