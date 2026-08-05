import { Mark, mergeAttributes } from '@tiptap/core'

// Anchors a comment to a text range. Visual is a side icon only — no text
// recolor/highlight (see main.css `.comment-anchor`).
export const CommentMark = Mark.create({
  name: 'comment',
  excludes: '',
  // Without this, pressing Enter with the caret at the end of a commented
  // range carries the mark onto the new paragraph — which then renders the
  // icon on an empty line that has nothing to do with the comment. The mark
  // should stop at its own boundary.
  keepOnSplit: false,

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
