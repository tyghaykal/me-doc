<template>
  <footer class="border-t border-slate-200 dark:border-slate-800">
    <div class="mx-auto flex max-w-5xl flex-col items-center justify-between gap-4 px-6 py-8 sm:flex-row">
      <span class="text-sm font-semibold text-slate-900 dark:text-slate-100">me-doc</span>
      <p class="text-sm text-slate-500 dark:text-slate-400">
        &copy; {{ new Date().getFullYear() }} me-doc. All rights reserved.
      </p>
      <div class="flex items-center gap-4 text-sm">
        <NuxtLink to="/login" class="text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100">Log in</NuxtLink>
        <NuxtLink to="/register" class="text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100">Sign up</NuxtLink>
      </div>
    </div>
  </footer>
</template>
