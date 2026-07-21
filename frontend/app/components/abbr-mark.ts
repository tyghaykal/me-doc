import { Mark, mergeAttributes } from '@tiptap/core'

export const AbbrMark = Mark.create({
  name: 'abbr',

  addAttributes() {
    return {
      title: {
        default: null,
        parseHTML: (el) => el.getAttribute('title'),
        renderHTML: (attrs) => (attrs.title ? { title: attrs.title } : {}),
      },
    }
  },

  parseHTML() {
    return [{ tag: 'abbr' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['abbr', mergeAttributes(HTMLAttributes), 0]
  },
})
