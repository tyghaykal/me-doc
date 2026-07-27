<script setup lang="ts">
import { parseMermaid } from '~/utils/diagram/mermaid'

const props = defineProps<{
  modelValue: string
  readonly?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [string] }>()

const error = ref<string | null>(null)
let timer: ReturnType<typeof setTimeout> | undefined

function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLTextAreaElement).value)
}

async function validate(src: string) {
  error.value = await parseMermaid(src)
}
watch(
  () => props.modelValue,
  (src) => {
    clearTimeout(timer)
    timer = setTimeout(() => validate(src), 250)
  },
  { immediate: true },
)
onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <div class="flex h-full min-h-0 flex-col bg-white dark:bg-neutral-900">
    <textarea
      :value="modelValue"
      :readonly="readonly"
      spellcheck="false"
      autocapitalize="off"
      autocorrect="off"
      placeholder="graph TD&#10;  A[Start] --> B[Next]"
      class="min-h-0 flex-1 resize-none bg-transparent px-4 py-3 font-mono text-[13px] leading-relaxed text-neutral-800 outline-none placeholder:text-neutral-400 dark:text-neutral-200 dark:placeholder:text-neutral-600"
      @input="onInput"
    />
    <div
      class="flex items-center gap-2 border-t border-neutral-200 px-4 py-1.5 text-xs dark:border-neutral-800"
      :class="error ? 'text-red-600 dark:text-red-400' : 'text-neutral-400 dark:text-neutral-600'"
    >
      <span
        class="inline-block h-1.5 w-1.5 shrink-0 rounded-full"
        :class="error ? 'bg-red-500' : 'bg-emerald-500'"
      />
      <span class="truncate">{{ error || 'Valid Mermaid syntax' }}</span>
    </div>
  </div>
</template>
