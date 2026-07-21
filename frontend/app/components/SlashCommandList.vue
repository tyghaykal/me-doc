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
    class="w-80 max-h-[min(22rem,calc(100vh-1rem))] overflow-y-auto rounded-xl border border-neutral-200 bg-white py-1.5 text-sm shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
  >
    <div v-for="g in groups" :key="g.group">
      <p class="px-3 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">
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
            ? 'bg-neutral-200 dark:bg-neutral-800'
            : 'hover:bg-neutral-50 dark:hover:bg-neutral-800/70'
        "
        @click="selectItem(index)"
        @mouseenter="selectedIndex = index"
      >
        <span
          class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border text-[13px] font-semibold"
          :class="
            index === selectedIndex
              ? 'border-neutral-300 bg-white text-neutral-900 dark:border-neutral-600 dark:bg-neutral-900 dark:text-neutral-100'
              : 'border-neutral-200 bg-neutral-50 text-neutral-600 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300'
          "
        >
          {{ item.icon }}
        </span>
        <span class="min-w-0 flex-1">
          <span
            class="block truncate font-medium"
            :class="index === selectedIndex ? 'text-neutral-900 dark:text-white' : 'text-neutral-800 dark:text-neutral-100'"
          >
            {{ item.title }}
          </span>
          <span class="block truncate text-xs text-neutral-500 dark:text-neutral-400">
            {{ item.description }}
          </span>
        </span>
      </button>
    </div>
  </div>
  <div
    v-else
    class="w-80 rounded-xl border border-neutral-200 bg-white px-3 py-3 text-sm text-neutral-400 shadow-xl dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-500"
  >
    No results
  </div>
</template>
