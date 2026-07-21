<script setup lang="ts">
import type { Editor } from '@tiptap/vue-3'

export type TocHeading = {
  id: string
  level: number
  text: string
  pos: number
}

const props = defineProps<{
  editor: Editor | null | undefined
  /** Scroll container that wraps the editor (the main column). */
  scrollRoot?: HTMLElement | null
}>()

const headings = ref<TocHeading[]>([])
const activeId = ref<string | null>(null)

function extractHeadings(editor: Editor): TocHeading[] {
  const items: TocHeading[] = []
  editor.state.doc.descendants((node, pos) => {
    if (node.type.name !== 'heading') return
    const text = node.textContent.trim()
    if (!text) return
    items.push({
      id: `h-${pos}`,
      level: node.attrs.level as number,
      text,
      pos,
    })
  })
  return items
}

function refreshHeadings() {
  const editor = props.editor
  if (!editor || editor.isDestroyed) {
    headings.value = []
    activeId.value = null
    return
  }
  headings.value = extractHeadings(editor)
  updateActiveFromScroll()
}

function headingEl(pos: number): HTMLElement | null {
  const editor = props.editor
  if (!editor || editor.isDestroyed) return null
  const dom = editor.view.nodeDOM(pos)
  return dom instanceof HTMLElement ? dom : null
}

function scrollToHeading(item: TocHeading) {
  const el = headingEl(item.pos)
  if (!el) return
  activeId.value = item.id
  el.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function updateActiveFromScroll() {
  if (headings.value.length === 0) {
    activeId.value = null
    return
  }

  const root = props.scrollRoot
  const topLine = root
    ? root.getBoundingClientRect().top + root.clientHeight * 0.15
    : window.innerHeight * 0.15

  let current: string | null = headings.value[0]?.id ?? null
  for (const h of headings.value) {
    const el = headingEl(h.pos)
    if (!el) continue
    if (el.getBoundingClientRect().top <= topLine) {
      current = h.id
    } else {
      break
    }
  }
  activeId.value = current
}

function onScroll() {
  updateActiveFromScroll()
}

let detachEditor: (() => void) | null = null

function attachEditor(editor: Editor | null | undefined) {
  detachEditor?.()
  detachEditor = null
  if (!editor || editor.isDestroyed) {
    headings.value = []
    activeId.value = null
    return
  }

  const onUpdate = () => refreshHeadings()
  editor.on('update', onUpdate)
  // Initial page content is fetched async — delayed rescans cover the race.
  const t1 = window.setTimeout(refreshHeadings, 300)
  const t2 = window.setTimeout(refreshHeadings, 1200)
  detachEditor = () => {
    editor.off('update', onUpdate)
    window.clearTimeout(t1)
    window.clearTimeout(t2)
  }
  refreshHeadings()
}

watch(
  () => props.editor,
  (ed) => attachEditor(ed),
  { immediate: true },
)

watch(
  () => props.scrollRoot,
  (root, prev) => {
    prev?.removeEventListener('scroll', onScroll)
    root?.addEventListener('scroll', onScroll, { passive: true })
    updateActiveFromScroll()
  },
  { immediate: true },
)

onMounted(() => {
  window.addEventListener('resize', updateActiveFromScroll, { passive: true })
})

onBeforeUnmount(() => {
  detachEditor?.()
  props.scrollRoot?.removeEventListener('scroll', onScroll)
  window.removeEventListener('resize', updateActiveFromScroll)
})

function indentClass(level: number): string {
  const step = Math.max(0, Math.min(level, 6) - 1)
  return (['pl-0', 'pl-2', 'pl-4', 'pl-6', 'pl-8', 'pl-10'][step] ?? 'pl-0')
}
</script>

<template>
  <!--
    Lives inside the document scroll area (sibling of the editor), not a
    separate app-shell column. Sticky so it tracks scroll with the page.
    Hidden entirely when there are no headings.
  -->
  <nav
    v-if="headings.length > 0"
    class="sticky top-8 hidden max-h-[calc(100vh-6rem)] w-44 shrink-0 self-start overflow-y-auto thin-scrollbar xl:block 2xl:w-52"
    aria-label="Table of contents"
  >
    <p class="mb-2 text-[11px] font-medium uppercase tracking-wide text-neutral-400 dark:text-neutral-500">
      On this page
    </p>
    <ul class="space-y-0.5 border-l border-neutral-200 dark:border-neutral-700">
      <li v-for="h in headings" :key="h.id">
        <button
          type="button"
          class="-ml-px w-full truncate border-l-2 py-1 pl-3 text-left text-xs leading-5 transition-colors"
          :class="[
            indentClass(h.level),
            h.id === activeId
              ? 'border-neutral-800 font-medium text-neutral-900 dark:border-neutral-200 dark:text-neutral-100'
              : 'border-transparent text-neutral-400 hover:text-neutral-700 dark:text-neutral-500 dark:hover:text-neutral-300',
          ]"
          :title="h.text"
          @click="scrollToHeading(h)"
        >
          {{ h.text }}
        </button>
      </li>
    </ul>
  </nav>
</template>
