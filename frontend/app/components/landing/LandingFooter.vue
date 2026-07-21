<template>
  <footer class="border-t border-neutral-200 dark:border-neutral-800">
    <div class="mx-auto flex max-w-5xl flex-col items-center justify-between gap-4 px-6 py-8 sm:flex-row">
      <span class="text-sm font-semibold text-neutral-900 dark:text-neutral-100">me-doc</span>
      <p class="text-sm text-neutral-500 dark:text-neutral-400">
        &copy; {{ new Date().getFullYear() }} me-doc. All rights reserved.
      </p>
      <div class="flex items-center gap-4 text-sm">
        <NuxtLink to="/login" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Log in</NuxtLink>
        <NuxtLink to="/register" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Sign up</NuxtLink>
      </div>
    </div>
  </footer>
</template>
