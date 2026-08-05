import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import { TextStyle } from '@tiptap/extension-text-style'
import Color from '@tiptap/extension-color'
import Highlight from '@tiptap/extension-highlight'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import { TableKit } from '@tiptap/extension-table'
import Subscript from '@tiptap/extension-subscript'
import Superscript from '@tiptap/extension-superscript'
import { SlashCommand } from '~/components/slash-command'
import { CommentMark } from '~/components/comment-mark'
import { DefinitionList, DefinitionTerm, DefinitionDescription } from '~/components/definition-list'
import { AbbrMark } from '~/components/abbr-mark'
import { ContainerNode } from '~/components/container-node'
import { DiagramNode } from '~/components/diagram-node'
import { DiagramEmbed } from '~/components/diagram-embed'

// Every editing extension that isn't tied to a collab room, shared by the
// collaborative Editor.vue (which layers Collaboration/CollaborationCaret on
// top) and the offline LocalEditor.vue. Callers that have no server workspace
// omit onEmbedDiagram, which drops the "Embed diagram" slash item.
export function useEditorExtensions(opts: {
  undoRedo?: boolean
  onInsertImage: () => void
  onEmbedDiagram?: () => void
  /** Enable the comment mark — server-backed pages only. */
  comments?: boolean
}) {
  return [
    StarterKit.configure({ undoRedo: opts.undoRedo ?? true }),
    Image.configure({ resize: { enabled: true, minWidth: 80, minHeight: 80 } }),
    TextStyle,
    Color,
    Highlight.configure({ multicolor: true }),
    TaskList,
    TaskItem.configure({ nested: true }),
    TableKit.configure({ table: { resizable: true } }),
    Subscript,
    Superscript,
    DefinitionList,
    DefinitionTerm,
    DefinitionDescription,
    AbbrMark,
    ContainerNode,
    DiagramNode,
    DiagramEmbed,
    ...(opts.comments ? [CommentMark] : []),
    SlashCommand.configure({ onInsertImage: opts.onInsertImage, onEmbedDiagram: opts.onEmbedDiagram }),
  ]
}
