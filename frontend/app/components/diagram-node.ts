import { Node, mergeAttributes } from '@tiptap/core'
import { VueNodeViewRenderer } from '@tiptap/vue-3'
import DiagramNodeView from './DiagramNodeView.vue'
import { DEFAULT_DIAGRAM_SOURCE } from '~/utils/diagram/templates'

// An inline Mermaid diagram block. Atom leaf: its Mermaid source lives in the
// `source` attribute (synced via the host page's Yjs doc), rendered/edited by a
// Vue NodeView. The backend export walker matches on this node's `source` attr.
export const DiagramNode = Node.create({
  name: 'diagram',
  group: 'block',
  atom: true,
  draggable: true,
  selectable: true,

  addAttributes() {
    return {
      source: {
        default: DEFAULT_DIAGRAM_SOURCE,
        parseHTML: (el) => el.getAttribute('data-source') || '',
        renderHTML: (attrs) => ({ 'data-source': attrs.source }),
      },
    }
  },

  parseHTML() {
    return [{ tag: 'div[data-diagram]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['div', mergeAttributes(HTMLAttributes, { 'data-diagram': '' })]
  },

  addNodeView() {
    return VueNodeViewRenderer(DiagramNodeView)
  },
})
