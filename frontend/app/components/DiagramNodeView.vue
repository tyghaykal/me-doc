<script setup lang="ts">
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'

const props = defineProps(nodeViewProps)

// The Mermaid source is a node attribute — it rides the host page's Yjs doc, so
// it syncs and persists with the document for free.
// ponytail: node attributes are last-writer-wins, so simultaneous character
// edits of an *inline* diagram by two people don't char-merge (standalone
// diagram pages use a Y.Text and do). Fine — inline blocks are edited solo.
const source = computed<string>({
  get: () => props.node.attrs.source ?? '',
  set: (v) => props.updateAttributes({ source: v }),
})
</script>

<template>
  <NodeViewWrapper
    class="diagram-node my-3"
    :contenteditable="false"
    @keydown.stop
  >
    <DiagramEditor
      v-model:source="source"
      :readonly="!props.editor.isEditable"
      variant="inline"
    />
  </NodeViewWrapper>
</template>
