<script setup lang="ts">
import type { PresenceUser } from '~/composables/useCollab'
import { DIAGRAM_TEMPLATES, type DiagramTemplate } from '~/utils/diagram/templates'

type View = 'code' | 'split' | 'diagram'

defineProps<{
  view: View
  presence?: PresenceUser[]
  readonly?: boolean
  fullscreen?: boolean
}>()
const emit = defineEmits<{
  'update:view': [View]
  insert: [DiagramTemplate]
  export: []
  'toggle-fullscreen': []
}>()

const views: { id: View; label: string; icon: string }[] = [
  { id: 'code', label: 'Code', icon: '{ }' },
  { id: 'split', label: 'Split', icon: '▥' },
  { id: 'diagram', label: 'Diagram', icon: '◈' },
]

const templatesOpen = ref(false)
function pick(t: DiagramTemplate) {
  emit('insert', t)
  templatesOpen.value = false
}
</script>

<template>
  <div class="flex items-center gap-2 border-b border-neutral-200 bg-white px-2 py-1.5 dark:border-neutral-800 dark:bg-neutral-900">
    <!-- View toggle -->
    <div class="flex overflow-hidden rounded-lg border border-neutral-200 dark:border-neutral-700">
      <button
        v-for="v in views"
        :key="v.id"
        type="button"
        class="px-2.5 py-1 text-xs font-medium transition-colors"
        :class="
          view === v.id
            ? 'bg-neutral-900 text-white dark:bg-neutral-100 dark:text-neutral-900'
            : 'text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800'
        "
        @click="emit('update:view', v.id)"
      >
        <span class="mr-1 font-mono">{{ v.icon }}</span>{{ v.label }}
      </button>
    </div>

    <!-- Templates -->
    <div v-if="!readonly" class="relative">
      <button
        type="button"
        class="rounded-lg px-2.5 py-1 text-xs font-medium text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
        @click="templatesOpen = !templatesOpen"
      >
        + Type
      </button>
      <div
        v-if="templatesOpen"
        class="absolute left-0 top-full z-30 mt-1 w-44 overflow-hidden rounded-lg border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-800"
        @pointerleave="templatesOpen = false"
      >
        <button
          v-for="t in DIAGRAM_TEMPLATES"
          :key="t.label"
          type="button"
          class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-200 dark:hover:bg-neutral-700"
          @click="pick(t)"
        >
          <span class="w-4 text-center text-neutral-400">{{ t.icon }}</span>{{ t.label }}
        </button>
      </div>
    </div>

    <div class="flex-1" />

    <!-- Presence -->
    <div v-if="presence && presence.length" class="flex -space-x-1.5">
      <div
        v-for="u in presence.slice(0, 4)"
        :key="u.clientId"
        class="flex h-6 w-6 items-center justify-center rounded-full border-2 border-white text-[10px] font-semibold text-white dark:border-neutral-900"
        :style="{ backgroundColor: u.color }"
        :title="u.name"
      >
        <img v-if="u.avatarUrl" :src="u.avatarUrl" :alt="u.name" class="h-full w-full rounded-full object-cover" />
        <span v-else>{{ (u.name[0] || '?').toUpperCase() }}</span>
      </div>
    </div>

    <!-- Export -->
    <button
      type="button"
      class="rounded-lg px-2.5 py-1 text-xs font-medium text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
      title="Download SVG"
      @click="emit('export')"
    >
      ↓ SVG
    </button>

    <!-- Fullscreen -->
    <button
      type="button"
      class="rounded-lg px-2.5 py-1 text-xs font-medium text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
      :title="fullscreen ? 'Exit fullscreen' : 'Fullscreen'"
      @click="emit('toggle-fullscreen')"
    >
      {{ fullscreen ? '⤡' : '⤢' }}
    </button>
  </div>
</template>
