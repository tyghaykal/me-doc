import { Node, mergeAttributes } from '@tiptap/core'

export const DefinitionList = Node.create({
  name: 'definitionList',
  group: 'block',
  content: '(definitionTerm definitionDescription+)+',

  parseHTML() {
    return [{ tag: 'dl' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['dl', mergeAttributes(HTMLAttributes), 0]
  },
})

export const DefinitionTerm = Node.create({
  name: 'definitionTerm',
  content: 'inline*',

  parseHTML() {
    return [{ tag: 'dt' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['dt', mergeAttributes(HTMLAttributes), 0]
  },
})

export const DefinitionDescription = Node.create({
  name: 'definitionDescription',
  content: 'inline*',

  parseHTML() {
    return [{ tag: 'dd' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['dd', mergeAttributes(HTMLAttributes), 0]
  },
})
