<script setup lang="ts">
const features = [
  {
    title: 'Real-time collaboration',
    description: 'Edit the same page together, live — everyone sees changes as they happen.',
    path: 'M17 20h5v-2a4 4 0 0 0-3-3.87M9 20H4v-2a4 4 0 0 1 3-3.87m5-5.13a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm-6 9v-2a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v2',
  },
  {
    title: 'Rich block editor',
    description: 'Headings, lists, code blocks, and images — a clean writing experience.',
    path: 'M11 4h9M11 8h6M11 12h9M11 16h6M4 4h.01M4 8l3 3-3 3',
  },
  {
    title: 'Sharing & permissions',
    description: 'Invite people as viewers or editors, or generate a public link in one click.',
    path: 'M8.68 13.34a3 3 0 1 0 0-2.68m0 2.68a3 3 0 1 1 0-2.68m0 2.68 6.64 3.83m0-9.86-6.64 3.85M19 5a3 3 0 1 1-6 0 3 3 0 0 1 6 0zm0 14a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
  },
  {
    title: 'Export anywhere',
    description: 'Take a page with you as Markdown, Word (.docx), or PDF, any time.',
    path: 'M12 3v12m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2',
  },
]
</script>

<template>
  <section class="mx-auto max-w-5xl px-6 py-16 sm:py-20">
    <div class="grid grid-cols-1 gap-10 lg:grid-cols-5 lg:gap-16">
      <!-- Real-time collaboration leads — it's the product's core differentiator, not a
           peer of the other three, so it gets its own weight instead of a fourth identical card. -->
      <div class="lg:col-span-3">
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
          <path :d="features[0].path" />
        </svg>
        <h3 class="mt-5 text-2xl font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
          {{ features[0].title }}
        </h3>
        <p class="mt-3 max-w-md text-base text-neutral-600 dark:text-neutral-400">
          {{ features[0].description }}
        </p>
      </div>

      <div class="flex flex-col gap-6 border-t border-neutral-200 pt-8 lg:col-span-2 lg:border-l lg:border-t-0 lg:pl-10 lg:pt-0 dark:border-neutral-800">
        <div v-for="f in features.slice(1)" :key="f.title" class="flex gap-3">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="mt-0.5 h-5 w-5 shrink-0 text-neutral-500 dark:text-neutral-400"
          >
            <path :d="f.path" />
          </svg>
          <div>
            <h4 class="text-sm font-semibold text-neutral-900 dark:text-neutral-100">{{ f.title }}</h4>
            <p class="mt-1 text-sm text-neutral-600 dark:text-neutral-400">{{ f.description }}</p>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
