<script setup lang="ts">
import type { PresenceUser } from '~/composables/useCollab'
import { renderMermaid } from '~/utils/diagram/mermaid'
import { adapterForSource } from '~/utils/diagram/adapters'
import type { DiagramTemplate } from '~/utils/diagram/templates'

type View = 'code' | 'split' | 'diagram'

const props = withDefaults(
  defineProps<{
    source: string
    readonly?: boolean
    presence?: PresenceUser[]
    /** 'page' fills its container; 'inline' is a fixed-height embedded block. */
    variant?: 'page' | 'inline'
  }>(),
  { variant: 'page' },
)
const emit = defineEmits<{ 'update:source': [string] }>()

const { isDark } = useTheme()
const view = ref<View>('split')

const model = computed({
  get: () => props.source,
  set: (v: string) => emit('update:source', v),
})

// Interactive drag-drop canvas is available only when an adapter both supports
// the diagram type AND can parse this specific source (i.e. it can round-trip
// safely). Otherwise — read-only diagrams, or flowcharts using subgraphs/styles
// the adapter can't regenerate — the Diagram tab shows a rendered preview.
const canEditVisually = computed(() => {
  if (props.readonly) return false
  return !!adapterForSource(props.source)?.parse(props.source)
})

function insertTemplate(t: DiagramTemplate) {
  if (props.source.trim() && !window.confirm('Replace the current diagram with a new template?')) return
  model.value = t.source
}

async function exportSvg() {
  const { svg } = await renderMermaid(props.source, isDark.value)
  if (!svg) return
  const blob = new Blob([svg], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'diagram.svg'
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <div
    class="flex flex-col overflow-hidden rounded-xl border border-neutral-200 dark:border-neutral-800"
    :class="variant === 'page' ? 'h-full' : 'h-[380px]'"
  >
    <DiagramToolbar
      :view="view"
      :presence="presence"
      :readonly="readonly"
      @update:view="view = $event"
      @insert="insertTemplate"
      @export="exportSvg"
    />

    <div class="grid min-h-0 flex-1" :class="view === 'split' ? 'grid-cols-2' : 'grid-cols-1'">
      <DiagramCodePane
        v-if="view !== 'diagram'"
        v-model="model"
        :readonly="readonly"
        class="min-w-0 border-r border-neutral-200 dark:border-neutral-800"
        :class="view === 'code' ? 'border-r-0' : ''"
      />
      <!-- Visual view: interactive canvas where an adapter supports it, else preview. -->
      <DiagramFlow
        v-if="view === 'diagram' && canEditVisually"
        v-model:source="model"
        class="min-w-0"
      />
      <div v-else-if="view !== 'code'" class="relative min-w-0">
        <DiagramCanvas :source="source" class="h-full" />
        <div
          v-if="view === 'diagram' && !readonly"
          class="pointer-events-none absolute inset-x-0 top-0 z-10 border-b border-amber-200 bg-amber-50/95 px-4 py-1.5 text-xs text-amber-800 dark:border-amber-900/50 dark:bg-amber-950/80 dark:text-amber-300"
        >
          Visual editing isn’t available for this diagram (it uses subgraphs, styles, or an
          unsupported type) — showing a preview. Edit it in the Code tab.
        </div>
      </div>
    </div>
  </div>
</template>
