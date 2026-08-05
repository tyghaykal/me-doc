<script setup lang="ts">
const releases = [
  {
    version: 'v0.2.0-alpha',
    entries: [
      {
        title: 'AI writing assistant (BYOK)',
        detail: 'Rephrase, fix grammar, reformat, proofread, or explain any selection — powered by your own OpenAI-compatible endpoint (OpenAI, OpenRouter, Groq, local Ollama). Configure URL, key, and model once in Settings; keys are encrypted at rest and never leave your server.',
      },
      {
        title: 'Offline / local documents',
        detail: 'Create and edit documents straight on your filesystem — no server page, no upload. Autosaves as plain Markdown (or HTML), with a Local section in the sidebar to reopen recent files. Nothing is shared or uploaded.',
      },
      {
        title: 'Notion-style title & body navigation',
        detail: 'Arrow keys move the caret between the title and the first line, and backspacing on an empty title with a body promotes the first line into it — select-all (Ctrl/Cmd+A) also stays scoped to the document.',
      },
      {
        title: 'Block menu & drag handle everywhere',
        detail: 'The line drag-handle and mark-text menu (formatting, colors, AI) now work in local documents too, with comments still reserved for shared pages.',
      },
      {
        title: 'White-label product name',
        detail: 'The product name is now a deploy-time setting (PRODUCT_NAME), shown in the browser tab, landing page, and emails — no more hardcoded branding.',
      },
      {
        title: 'Document import',
        detail: 'Bring in existing files and have them converted to editable pages — Word, PDF, Excel, PowerPoint, EPUB, HTML, and CSV, alongside plain text and Markdown.',
      },
      {
        title: 'Live title sync',
        detail: 'Renaming a page or diagram now shows up instantly for every other viewer, including anonymous link guests, who can now save their own renames as well.',
      },
      {
        title: 'Diagram viewer polish',
        detail: 'Diagrams auto-fit the canvas on open, plus a fullscreen toggle for working on larger flowcharts.',
      },
      {
        title: 'Automated database backups',
        detail: 'A scheduled backup job dumps Postgres, compresses it, and stores it in your backup bucket daily — with optional retention-based pruning.',
      },
      {
        title: 'End-to-end test suite',
        detail: 'A Playwright suite covering auth, collaboration, comments, imports, exports, settings, and local documents runs against the real stack.',
      },
    ],
  },
  {
    version: 'v0.1.0-alpha',
    entries: [
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
    ],
  },
]
</script>

<template>
  <div class="mx-auto w-full max-w-3xl">
    <h1 class="mb-6 text-2xl font-bold text-neutral-900 dark:text-neutral-100">Changelog</h1>

    <div v-for="(release, i) in releases" :key="release.version" :class="i > 0 ? 'mt-8' : ''">
      <div class="mb-3 flex items-center gap-2">
        <span class="rounded-full bg-teal-600 px-2.5 py-0.5 text-xs font-semibold text-white dark:bg-teal-500 dark:text-neutral-950">
          {{ release.version }}
        </span>
        <span v-if="i === 0" class="text-xs text-neutral-400 dark:text-neutral-500">Current release</span>
      </div>

      <ul class="divide-y divide-neutral-200 rounded-lg border border-neutral-200 dark:divide-neutral-800 dark:border-neutral-800">
        <li v-for="entry in release.entries" :key="entry.title" class="bg-white px-4 py-3 dark:bg-neutral-900">
          <p class="font-medium text-neutral-900 dark:text-neutral-100">{{ entry.title }}</p>
          <p class="mt-0.5 text-sm text-neutral-500 dark:text-neutral-400">{{ entry.detail }}</p>
        </li>
      </ul>
    </div>
  </div>
</template>
