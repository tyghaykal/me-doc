<script setup lang="ts">
import * as Y from 'yjs'
import type { PresenceUser } from '~/composables/useCollab'
import { bindYText } from '~/utils/diagram/ytext'
import { DEFAULT_PAGE_ICON } from '~/stores/pages'

const props = defineProps<{
  pageId: string
  workspaceId: string
  title: string
  icon?: string | null
  linkToken?: string | null
  readOnly?: boolean
}>()
const emit = defineEmits<{ 'presence-change': [PresenceUser[]] }>()

const api = useApi()
const pagesStore = usePagesStore()

// Collaborative doc over the shared /ws/pages/:id room. Mermaid source lives in
// a Y.Text so concurrent edits merge; the same doc persists via the collab
// server just like a document page.
const { doc, presence } = useCollab({
  pageId: props.pageId,
  linkToken: props.linkToken,
  announce: !props.readOnly,
})
const ytext = doc.getText('source')
const source = ref('')
bindYText(ytext, source)

watch(presence, (p) => emit('presence-change', [...p]), { immediate: true, deep: true })

// The collab flusher persists yjs_state but not plain_text; a debounced REST
// save keeps the search index (Mermaid source) current — same split as Editor.
let saveTimer: ReturnType<typeof setTimeout> | undefined
async function save() {
  const update = Y.encodeStateAsUpdate(doc)
  await api(`/pages/${props.pageId}/content`, {
    method: 'PUT',
    body: update,
    headers: { 'Content-Type': 'application/octet-stream' },
    query: props.linkToken ? { link: props.linkToken } : undefined,
  }).catch(() => {})
}
watch(source, () => {
  if (props.readOnly) return
  clearTimeout(saveTimer)
  saveTimer = setTimeout(save, 1500)
})
onBeforeUnmount(() => clearTimeout(saveTimer))

// --- Title / icon (mirrors Editor.vue) ---
const titleDraft = ref(props.title)
watch(() => props.title, (t) => (titleDraft.value = t))
let titleTimer: ReturnType<typeof setTimeout> | undefined
function scheduleTitleSave() {
  clearTimeout(titleTimer)
  titleTimer = setTimeout(
    () => pagesStore.updatePage(props.pageId, { title: titleDraft.value || 'Untitled' }),
    800,
  )
}

const iconDraft = ref(props.icon ?? '')
const iconPickerOpen = ref(false)
watch(() => props.icon, (i) => (iconDraft.value = i ?? ''))
const EMOJI_CHOICES = ['📊', '📈', '📉', '🗂️', '🧭', '🔀', '🧩', '🗺️', '⚙️', '🕸️', '🔗', '📐']
function setIcon(icon: string | null) {
  iconDraft.value = icon ?? ''
  iconPickerOpen.value = false
  pagesStore.updatePage(props.pageId, { icon })
}
</script>

<template>
  <div class="flex h-full min-h-0 w-full flex-col">
    <div class="relative mb-1 inline-block">
      <button
        type="button"
        :disabled="readOnly"
        class="rounded px-1 text-4xl leading-none enabled:hover:bg-neutral-100 dark:enabled:hover:bg-neutral-800"
        @click="iconPickerOpen = !iconPickerOpen"
      >
        {{ iconDraft || DEFAULT_PAGE_ICON }}
      </button>
      <template v-if="iconPickerOpen && !readOnly">
        <div class="fixed inset-0 z-40" @click="iconPickerOpen = false" />
        <div class="absolute left-0 z-50 mt-1 grid w-64 grid-cols-8 gap-1 rounded-md border border-neutral-200 bg-white p-2 shadow-lg dark:border-neutral-700 dark:bg-neutral-900">
          <button
            v-for="e in EMOJI_CHOICES"
            :key="e"
            type="button"
            class="rounded p-1 text-xl hover:bg-neutral-100 dark:hover:bg-neutral-800"
            @click="setIcon(e)"
          >
            {{ e }}
          </button>
          <button
            v-if="iconDraft"
            type="button"
            class="col-span-8 mt-1 rounded px-2 py-1 text-left text-xs text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
            @click="setIcon(null)"
          >
            Remove icon
          </button>
        </div>
      </template>
    </div>

    <input
      v-model="titleDraft"
      type="text"
      placeholder="Untitled diagram"
      :readonly="readOnly"
      class="mb-3 w-full border-none bg-transparent text-4xl font-bold text-neutral-900 outline-none placeholder:text-neutral-300 dark:text-neutral-100 dark:placeholder:text-neutral-700"
      @input="!readOnly && scheduleTitleSave()"
      @blur="!readOnly && pagesStore.updatePage(pageId, { title: titleDraft || 'Untitled' })"
    />

    <div class="min-h-0 flex-1">
      <DiagramEditor v-model:source="source" :readonly="readOnly" :presence="presence" variant="page" />
    </div>
  </div>
</template>
