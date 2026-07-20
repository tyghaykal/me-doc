<script setup lang="ts">
import type { SlashCommandItem } from './slash-command'

const props = defineProps<{ items: SlashCommandItem[]; command: (item: SlashCommandItem) => void }>()

const selectedIndex = ref(0)
const listEl = ref<HTMLElement | null>(null)

const groups = computed(() => {
  const map = new Map<string, { group: string; items: { item: SlashCommandItem; index: number }[] }>()
  props.items.forEach((item, index) => {
    const g = item.group || 'Other'
    if (!map.has(g)) map.set(g, { group: g, items: [] })
    map.get(g)!.items.push({ item, index })
  })
  return Array.from(map.values())
})

watch(
  () => props.items,
  () => {
    selectedIndex.value = 0
  },
)

watch(selectedIndex, async () => {
  await nextTick()
  const el = listEl.value?.querySelector<HTMLElement>(`[data-index="${selectedIndex.value}"]`)
  el?.scrollIntoView({ block: 'nearest' })
})

function selectItem(index: number) {
  const item = props.items[index]
  if (item) props.command(item)
}

function onKeyDown({ event }: { event: KeyboardEvent }): boolean {
  if (!props.items.length) return false
  if (event.key === 'ArrowUp') {
    selectedIndex.value = (selectedIndex.value + props.items.length - 1) % props.items.length
    return true
  }
  if (event.key === 'ArrowDown') {
    selectedIndex.value = (selectedIndex.value + 1) % props.items.length
    return true
  }
  if (event.key === 'Enter') {
    selectItem(selectedIndex.value)
    return true
  }
  return false
}

defineExpose({ onKeyDown })
</script>

<template>
  <div
    v-if="items.length"
    ref="listEl"
    role="menu"
    class="w-80 max-h-[min(22rem,calc(100vh-1rem))] overflow-y-auto rounded-xl border border-slate-200 bg-white py-1.5 text-sm shadow-xl dark:border-slate-700 dark:bg-slate-900"
  >
    <div v-for="g in groups" :key="g.group">
      <p class="px-3 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
        {{ g.group }}
      </p>
      <button
        v-for="{ item, index } in g.items"
        :key="item.title"
        type="button"
        role="menuitem"
        :data-index="index"
        class="flex w-full items-center gap-3 px-2.5 py-1.5 text-left"
        :class="
          index === selectedIndex
            ? 'bg-indigo-50 dark:bg-indigo-950/50'
            : 'hover:bg-slate-50 dark:hover:bg-slate-800/70'
        "
        @click="selectItem(index)"
        @mouseenter="selectedIndex = index"
      >
        <span
          class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border text-[13px] font-semibold"
          :class="
            index === selectedIndex
              ? 'border-indigo-200 bg-white text-indigo-600 dark:border-indigo-800 dark:bg-slate-900 dark:text-indigo-300'
              : 'border-slate-200 bg-slate-50 text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300'
          "
        >
          {{ item.icon }}
        </span>
        <span class="min-w-0 flex-1">
          <span
            class="block truncate font-medium"
            :class="index === selectedIndex ? 'text-indigo-900 dark:text-indigo-100' : 'text-slate-800 dark:text-slate-100'"
          >
            {{ item.title }}
          </span>
          <span class="block truncate text-xs text-slate-500 dark:text-slate-400">
            {{ item.description }}
          </span>
        </span>
      </button>
    </div>
  </div>
  <div
    v-else
    class="w-80 rounded-xl border border-slate-200 bg-white px-3 py-3 text-sm text-slate-400 shadow-xl dark:border-slate-700 dark:bg-slate-900 dark:text-slate-500"
  >
    No results
  </div>
</template>
