<script setup lang="ts">
import { renderMermaid } from '~/utils/diagram/mermaid'

const props = defineProps<{
  source: string
  /** Reserved for the interactive Vue Flow layer (Phase C); preview for now. */
  interactive?: boolean
}>()

const { isDark } = useTheme()

const host = ref<HTMLElement | null>(null)
const error = ref<string | null>(null)
const rendering = ref(false)

// Pan/zoom state for the rendered SVG.
const scale = ref(1)
const tx = ref(0)
const ty = ref(0)

let renderTimer: ReturnType<typeof setTimeout> | undefined
let renderToken = 0

async function render() {
  const token = ++renderToken
  rendering.value = true
  const { svg, error: err } = await renderMermaid(props.source, isDark.value)
  // A newer render superseded this one — drop the stale result.
  if (token !== renderToken) return
  rendering.value = false
  if (err !== undefined) {
    error.value = err
    return
  }
  error.value = null
  if (host.value) host.value.innerHTML = svg ?? ''
}

function scheduleRender() {
  clearTimeout(renderTimer)
  renderTimer = setTimeout(render, 150)
}

watch(() => [props.source, isDark.value], scheduleRender, { immediate: true })
onBeforeUnmount(() => clearTimeout(renderTimer))

function zoomBy(factor: number) {
  scale.value = Math.min(4, Math.max(0.2, scale.value * factor))
}
function resetView() {
  scale.value = 1
  tx.value = 0
  ty.value = 0
}
function onWheel(e: WheelEvent) {
  e.preventDefault()
  zoomBy(e.deltaY < 0 ? 1.1 : 0.9)
}

// Drag to pan.
let panning = false
let startX = 0
let startY = 0
function onPointerDown(e: PointerEvent) {
  panning = true
  startX = e.clientX - tx.value
  startY = e.clientY - ty.value
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
}
function onPointerMove(e: PointerEvent) {
  if (!panning) return
  tx.value = e.clientX - startX
  ty.value = e.clientY - startY
}
function onPointerUp() {
  panning = false
}

defineExpose({ resetView, zoomBy })
</script>

<template>
  <div class="diagram-canvas relative h-full w-full overflow-hidden bg-neutral-50 dark:bg-neutral-950">
    <!-- Zoom controls -->
    <div class="absolute right-3 top-3 z-10 flex flex-col overflow-hidden rounded-lg border border-neutral-200 bg-white/90 shadow-sm backdrop-blur dark:border-neutral-800 dark:bg-neutral-900/90">
      <button
        type="button"
        class="px-2.5 py-1.5 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
        title="Zoom in"
        @click="zoomBy(1.2)"
      >+</button>
      <button
        type="button"
        class="border-t border-neutral-200 px-2.5 py-1.5 text-neutral-600 hover:bg-neutral-100 dark:border-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-800"
        title="Zoom out"
        @click="zoomBy(1 / 1.2)"
      >−</button>
      <button
        type="button"
        class="border-t border-neutral-200 px-2.5 py-1.5 text-[11px] text-neutral-600 hover:bg-neutral-100 dark:border-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-800"
        title="Reset view"
        @click="resetView"
      >Fit</button>
    </div>

    <!-- Error overlay -->
    <div
      v-if="error"
      class="pointer-events-none absolute inset-x-0 bottom-0 z-10 border-t border-red-200 bg-red-50/95 px-4 py-2 text-xs text-red-700 dark:border-red-900/50 dark:bg-red-950/80 dark:text-red-300"
    >
      <span class="font-medium">Can't render:</span> {{ error }}
    </div>

    <div
      class="h-full w-full cursor-grab touch-none select-none active:cursor-grabbing"
      @wheel="onWheel"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <div
        class="flex h-full w-full items-center justify-center"
        :style="{ transform: `translate(${tx}px, ${ty}px) scale(${scale})` }"
      >
        <div
          v-if="!source.trim()"
          class="text-sm text-neutral-400 dark:text-neutral-600"
        >
          Start typing Mermaid to see your diagram
        </div>
        <div v-else ref="host" class="diagram-svg [&_svg]:max-w-none" />
      </div>
    </div>
  </div>
</template>
