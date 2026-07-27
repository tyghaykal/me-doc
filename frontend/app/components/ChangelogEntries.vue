<script setup lang="ts">
const entries = [
  {
    title: 'Live collaboration',
    detail: 'Every document is a shared editing session — multiple people can write in the same page at once, with live cursors, selections, and presence avatars showing who else is viewing.',
  },
  {
    title: 'Brand refresh & UI polish',
    detail: 'New teal accent theme, smoother modal and sidebar transitions, and a fixed OTP login route.',
  },
  {
    title: 'Mermaid diagrams',
    detail: 'Dedicated diagram pages with a visual flowchart editor, code/split/diagram views, and inline diagram embeds inside documents.',
  },
  {
    title: 'Real-time comments',
    detail: 'Threaded comments anchored to text, streamed live over WebSockets so replies and resolutions appear instantly for everyone on the page.',
  },
  {
    title: 'Home & page browsing',
    detail: 'A dedicated home view listing root pages with pagination and drag-and-drop reordering, plus lazy-loaded child pages in the sidebar tree.',
  },
  {
    title: 'Notion-style layout & favorites',
    detail: 'Redesigned sidebar with recents, favorites, and shared pages, a top app bar, and a quick search palette (Cmd/Ctrl+K).',
  },
  {
    title: 'Workspaces & sharing',
    detail: 'Multi-workspace support with member roles, page sharing via link or invite, and user profile settings for display name, password, and avatar.',
  },
  {
    title: 'Rich text editor',
    detail: 'Task lists, tables, table of contents, text color & highlight, subscript/superscript, definition lists, and version history with restore.',
  },
]
</script>

<template>
  <div class="mx-auto w-full max-w-3xl">
    <div class="mb-6">
      <h1 class="text-2xl font-bold text-neutral-900 dark:text-neutral-100">Changelog</h1>
      <div class="mt-2 flex items-center gap-2">
        <span class="rounded-full bg-teal-600 px-2.5 py-0.5 text-xs font-semibold text-white dark:bg-teal-500 dark:text-neutral-950">
          v0.1.0
        </span>
        <span class="text-xs text-neutral-400 dark:text-neutral-500">Current release</span>
      </div>
    </div>

    <ul class="divide-y divide-neutral-200 rounded-lg border border-neutral-200 dark:divide-neutral-800 dark:border-neutral-800">
      <li v-for="entry in entries" :key="entry.title" class="bg-white px-4 py-3 dark:bg-neutral-900">
        <p class="font-medium text-neutral-900 dark:text-neutral-100">{{ entry.title }}</p>
        <p class="mt-0.5 text-sm text-neutral-500 dark:text-neutral-400">{{ entry.detail }}</p>
      </li>
    </ul>
  </div>
</template>
