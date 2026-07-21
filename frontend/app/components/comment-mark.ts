import { Mark, mergeAttributes } from '@tiptap/core'

// Anchors a comment to a text range the same way bold/italic marks anchor
// formatting — rides on Yjs's CRDT semantics for free, so the anchor stays
// attached to its characters through concurrent edits with no relative-
// position bookkeeping. `excludes: ''` (instead of the mark-type default)
// lets two different comments overlap the same text.
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
    return ['span', mergeAttributes(HTMLAttributes, { class: 'comment-highlight' }), 0]
  },
})
