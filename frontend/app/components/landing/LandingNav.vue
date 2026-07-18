<script setup lang="ts">
const { isDark, toggleTheme } = useTheme()
const authStore = useAuthStore()
const authReady = useAuthReady()
</script>

<template>
  <nav class="mx-auto flex max-w-5xl items-center justify-between px-6 py-5">
    <span class="text-lg font-semibold tracking-tight text-slate-900 dark:text-slate-100">me-doc</span>

    <div class="flex items-center gap-2">
      <button
        type="button"
        aria-label="Toggle theme"
        class="rounded p-1.5 text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
        @click="toggleTheme"
      >
        <svg v-if="isDark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
          <circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
        </svg>
        <svg v-else xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="h-4 w-4">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </button>

      <div v-if="!authReady" class="invisible h-8 w-[158px]" />
      <template v-else-if="authStore.isAuthenticated">
        <NuxtLink
          to="/app"
          class="rounded bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400"
        >
          Open App
        </NuxtLink>
      </template>
      <template v-else>
        <NuxtLink
          to="/login"
          class="rounded px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
        >
          Log in
        </NuxtLink>
        <NuxtLink
          to="/register"
          class="rounded bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400"
        >
          Get started
        </NuxtLink>
      </template>
    </div>
  </nav>
</template>
