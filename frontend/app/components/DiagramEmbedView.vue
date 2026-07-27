<script setup lang="ts">
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'

const props = defineProps(nodeViewProps)

const diagramId = computed<string>(() => props.node.attrs.diagramId ?? '')
const title = computed<string>(() => props.node.attrs.title || 'Diagram')

// Mirror the source diagram read-only over its collab room: `source` tracks the
// diagram's live Y.Text, so edits there re-render here without a save/reload.
const source = ref('')

if (import.meta.client && diagramId.value) {
  const { doc } = useCollab({ pageId: diagramId.value, announce: false })
  const ytext = doc.getText('source')
  const sync = () => (source.value = ytext.toString())
  sync()
  ytext.observe(sync)
  onBeforeUnmount(() => ytext.unobserve(sync))
}

function open() {
  navigateTo(`/app/${diagramId.value}`)
}
</script>

<template>
  <NodeViewWrapper class="diagram-embed my-3" :contenteditable="false">
    <div class="overflow-hidden rounded-xl border border-neutral-200 dark:border-neutral-800">
      <div class="flex items-center gap-2 border-b border-neutral-200 bg-neutral-50 px-3 py-1.5 dark:border-neutral-800 dark:bg-neutral-900/60">
        <span class="text-neutral-400">◈</span>
        <span class="truncate text-xs font-medium text-neutral-600 dark:text-neutral-300">{{ title }}</span>
        <span class="rounded bg-neutral-200 px-1.5 py-0.5 text-[10px] font-medium text-neutral-500 dark:bg-neutral-800 dark:text-neutral-400">live</span>
        <div class="flex-1" />
        <button
          type="button"
          class="rounded px-2 py-0.5 text-xs text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
          @click="open"
        >
          Open ↗
        </button>
      </div>
      <div class="h-[320px]">
        <DiagramCanvas :source="source" />
      </div>
    </div>
  </NodeViewWrapper>
</template>
