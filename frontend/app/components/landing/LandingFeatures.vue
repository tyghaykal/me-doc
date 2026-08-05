<script setup lang="ts">
const features = [
  {
    title: 'Real-time collaboration',
    description: 'Edit the same page together, live — everyone sees changes as they happen, with presence avatars showing who else is viewing and a shared undo stack.',
    path: 'M17 20h5v-2a4 4 0 0 0-3-3.87M9 20H4v-2a4 4 0 0 1 3-3.87m5-5.13a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm-6 9v-2a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v2',
  },
  {
    title: 'AI writing assistant',
    description: 'Rephrase, fix grammar, reformat, or explain any selection — powered by your own OpenAI-compatible endpoint (OpenAI, OpenRouter, Groq, local Ollama). Your key is encrypted at rest and never leaves your server.',
    path: 'M12 3l1.9 5.7L19.6 10l-5.7 1.9L12 17.6l-1.9-5.7L4.4 10l5.7-1.9zM19 15l.9 2.6L22.5 18l-2.6.9L19 21.5l-.9-2.6-2.6-.9 2.6-.9z',
  },
  {
    title: 'Offline & local documents',
    description: 'Write straight to your filesystem with autosave — plain Markdown on disk, a Local section in the sidebar to reopen recent files, and nothing uploaded to any server.',
    path: 'M12 15V3m0 12 4-4m-4 4-4-4M2 17l.6 2.4a2 2 0 0 0 1.9 1.5h15a2 2 0 0 0 1.9-1.5L22 17',
  },
  {
    title: 'Rich block editor',
    description: 'Headings, lists, tables, task lists, code blocks, images, and Mermaid diagrams — plus text color, highlight, subscript/superscript, and a table of contents.',
    path: 'M11 4h9M11 8h6M11 12h9M11 16h6M4 4h.01M4 8l3 3-3 3',
  },
  {
    title: 'Sharing & permissions',
    description: 'Invite people as viewers or editors, or generate a public link in one click. Every page supports role-based access and thread comments anchored to text.',
    path: 'M8.68 13.34a3 3 0 1 0 0-2.68m0 2.68a3 3 0 1 1 0-2.68m0 2.68 6.64 3.83m0-9.86-6.64 3.85M19 5a3 3 0 1 1-6 0 3 3 0 0 1 6 0zm0 14a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
  },
  {
    title: 'Export anywhere',
    description: 'Take a page with you as Markdown, Word (.docx), or PDF, any time — and bring existing files back in via document import.',
    path: 'M12 3v12m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2',
  },
]

// Left-side picker state: the selected feature's index drives the right panel.
const selected = ref(0)

// Auto-rotate through the features every few seconds so the section feels
// alive on load; the first click on a feature stops the rotation for good
// (the visitor is now driving, and a moving target under their cursor would
// only frustrate).
const rotating = ref(true)
let rotateTimer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  rotateTimer = setInterval(() => {
    if (rotating.value) selected.value = (selected.value + 1) % features.length
  }, 3500)
})
onBeforeUnmount(() => clearInterval(rotateTimer))

function selectFeature(i: number) {
  rotating.value = false
  selected.value = i
}
</script>

<template>
  <section id="features" class="mx-auto max-w-5xl px-6 py-16 sm:py-20">
    <h2 class="text-center text-3xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">
      Everything your team needs to write
    </h2>
    <p class="mx-auto mt-3 max-w-xl text-center text-neutral-600 dark:text-neutral-400">
      Pick a feature below to see what it does — it auto-rotates until you do.
    </p>

    <div class="mt-12 grid grid-cols-1 gap-10 lg:grid-cols-2 lg:gap-16">
      <!-- Left: clickable feature list -->
      <div class="flex flex-col gap-1.5">
        <button
          v-for="(f, i) in features"
          :key="f.title"
          type="button"
          class="flex items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors"
          :class="selected === i
            ? 'bg-teal-50 dark:bg-teal-950/40'
            : 'hover:bg-neutral-100 dark:hover:bg-neutral-800'"
          @click="selectFeature(i)"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-5 w-5 shrink-0"
            :class="selected === i ? 'text-teal-600 dark:text-teal-400' : 'text-neutral-500 dark:text-neutral-400'"
          >
            <path :d="f.path" />
          </svg>
          <span
            class="text-sm font-semibold"
            :class="selected === i ? 'text-teal-900 dark:text-teal-200' : 'text-neutral-800 dark:text-neutral-200'"
          >
            {{ f.title }}
          </span>
        </button>
      </div>

      <!-- Right: the selected feature's description -->
      <div class="flex flex-col justify-center border-t border-neutral-200 pt-8 lg:border-l lg:border-t-0 lg:pl-10 lg:pt-0 dark:border-neutral-800">
        <Transition
          mode="out-in"
          enter-active-class="transition duration-200 ease-out"
          enter-from-class="translate-y-1 opacity-0"
          enter-to-class="translate-y-0 opacity-100"
          leave-active-class="transition duration-150 ease-in"
          leave-from-class="translate-y-0 opacity-100"
          leave-to-class="translate-y-1 opacity-0"
        >
          <div :key="selected">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="h-9 w-9 text-teal-600 dark:text-teal-400"
            >
              <path :d="features[selected].path" />
            </svg>
            <h3 class="mt-5 text-2xl font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
              {{ features[selected].title }}
            </h3>
            <p class="mt-3 max-w-md text-base leading-relaxed text-neutral-600 dark:text-neutral-400">
              {{ features[selected].description }}
            </p>
          </div>
        </Transition>
      </div>
    </div>
  </section>
</template>
