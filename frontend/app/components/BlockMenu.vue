<script setup lang="ts">
const props = defineProps<{
  editor: any
  pos: number
  node: any
  x: number
  y: number
}>()

const emit = defineEmits<{ close: [] }>()

const textColors = [
  { name: 'Default', value: null },
  { name: 'Gray', value: '#787774' },
  { name: 'Brown', value: '#9F6B53' },
  { name: 'Orange', value: '#D9730D' },
  { name: 'Yellow', value: '#CB912F' },
  { name: 'Green', value: '#448361' },
  { name: 'Blue', value: '#337EA9' },
  { name: 'Purple', value: '#9065B0' },
  { name: 'Pink', value: '#C14C8A' },
  { name: 'Red', value: '#D44C47' },
]

const backgroundColors = [
  { name: 'Default', value: null },
  { name: 'Gray', value: '#F1F1EF' },
  { name: 'Brown', value: '#F4EEEE' },
  { name: 'Orange', value: '#FBECDD' },
  { name: 'Yellow', value: '#FBF3DB' },
  { name: 'Green', value: '#EDF3EC' },
  { name: 'Blue', value: '#E7F3F8' },
  { name: 'Purple', value: '#F6F3F9' },
  { name: 'Pink', value: '#FAF1F5' },
  { name: 'Red', value: '#FDEBEC' },
]

function blockRange() {
  return { from: props.pos, to: props.pos + props.node.nodeSize }
}

function contentRange() {
  return { from: props.pos + 1, to: props.pos + props.node.nodeSize - 1 }
}

function duplicate() {
  props.editor.chain().focus().insertContentAt(props.pos + props.node.nodeSize, props.node.toJSON()).run()
  emit('close')
}

function remove() {
  props.editor.chain().focus().deleteRange(blockRange()).run()
  emit('close')
}

function setTextColor(value: string | null) {
  const chain = props.editor.chain().focus().setTextSelection(contentRange())
  if (value) chain.setColor(value).run()
  else chain.unsetColor().run()
  emit('close')
}

function setBackgroundColor(value: string | null) {
  const chain = props.editor.chain().focus().setTextSelection(contentRange())
  if (value) chain.setHighlight({ color: value }).run()
  else chain.unsetHighlight().run()
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div class="fixed inset-0 z-40" @click="emit('close')" @contextmenu.prevent="emit('close')" />
    <div
      role="menu"
      class="fixed z-50 w-56 rounded-md border border-slate-200 bg-white py-1 text-sm shadow-lg dark:border-slate-700 dark:bg-slate-900"
      :style="{ left: `${x}px`, top: `${y}px` }"
    >
      <button
        type="button"
        role="menuitem"
        class="block w-full px-3 py-1.5 text-left text-slate-700 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-slate-800"
        @click="duplicate"
      >
        Duplicate
      </button>
      <button
        type="button"
        role="menuitem"
        class="block w-full px-3 py-1.5 text-left text-red-600 hover:bg-slate-50 dark:text-red-400 dark:hover:bg-slate-800"
        @click="remove"
      >
        Delete
      </button>

      <div class="my-1 border-t border-slate-200 dark:border-slate-800" />

      <p class="px-3 pb-1 pt-1 text-xs font-medium text-slate-400 dark:text-slate-500">Text color</p>
      <div class="flex flex-wrap gap-1 px-3 pb-2">
        <button
          v-for="c in textColors"
          :key="c.name"
          type="button"
          :title="c.name"
          class="h-5 w-5 rounded border border-slate-300 dark:border-slate-600"
          :style="{ color: c.value ?? 'inherit', backgroundColor: 'transparent' }"
          @click="setTextColor(c.value)"
        >
          A
        </button>
      </div>

      <p class="px-3 pb-1 text-xs font-medium text-slate-400 dark:text-slate-500">Background</p>
      <div class="flex flex-wrap gap-1 px-3 pb-2">
        <button
          v-for="c in backgroundColors"
          :key="c.name"
          type="button"
          :title="c.name"
          class="h-5 w-5 rounded border border-slate-300 dark:border-slate-600"
          :style="{ backgroundColor: c.value ?? 'transparent' }"
          @click="setBackgroundColor(c.value)"
        />
      </div>
    </div>
  </Teleport>
</template>
