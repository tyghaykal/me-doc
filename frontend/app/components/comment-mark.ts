import { Mark, mergeAttributes } from '@tiptap/core'

// Anchors a comment to a text range. Visual is a side icon only — no text
// recolor/highlight (see main.css `.comment-anchor`).
export const CommentMark = Mark.create({
  name: 'comment',
  excludes: '',

  addAttributes() {
    return {
      commentId: {
        default: null,
        parseHTML: (el) => el.getAttribute('data-comment-id'),
        renderHTML: (attrs) => ({ 'data-comment-id': attrs.commentId }),
      },
    }
  },

  parseHTML() {
    return [{ tag: 'span[data-comment-id]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', mergeAttributes(HTMLAttributes, { class: 'comment-anchor' }), 0]
  },
})
