import { Node, mergeAttributes } from '@tiptap/core'

// `::: warning ... :::` custom-container blocks (markdown-it-container style),
// rendered as a callout with distinct styling per type — see main.css.
export const ContainerNode = Node.create({
  name: 'container',
  group: 'block',
  content: 'block+',

  addAttributes() {
    return {
      containerType: {
        default: 'info',
        parseHTML: (el) => el.getAttribute('data-container') || 'info',
        renderHTML: (attrs) => ({ 'data-container': attrs.containerType }),
      },
    }
  },

  parseHTML() {
    return [{ tag: 'div[data-container]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['div', mergeAttributes(HTMLAttributes), 0]
  },
})
