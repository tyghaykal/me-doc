<script setup lang="ts">
const authStore = useAuthStore()
const authReady = useAuthReady()
</script>

<template>
  <footer class="border-t border-neutral-200 dark:border-neutral-800">
    <div class="mx-auto flex max-w-5xl flex-col items-center justify-between gap-4 px-6 py-8 sm:flex-row">
      <NuxtLink to="/" class="text-sm font-semibold text-neutral-900 dark:text-neutral-100">me-doc</NuxtLink>
      <p class="text-sm text-neutral-500 dark:text-neutral-400">
        &copy; {{ new Date().getFullYear() }} me-doc. All rights reserved.
      </p>
      <div class="flex items-center gap-4 text-sm">
        <NuxtLink to="/changelog" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Changelog</NuxtLink>
        <NuxtLink
          v-if="authReady && authStore.isAuthenticated"
          to="/app"
          class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100"
        >
          Open App
        </NuxtLink>
        <template v-else-if="authReady">
          <NuxtLink to="/login" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Log in</NuxtLink>
          <NuxtLink to="/register" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Sign up</NuxtLink>
        </template>
      </div>
    </div>
  </footer>
</template>
