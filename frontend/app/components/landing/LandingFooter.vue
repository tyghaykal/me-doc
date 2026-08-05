<script setup lang="ts">
const authStore = useAuthStore()
const authReady = useAuthReady()
const { public: { productName } } = useRuntimeConfig()
</script>

<template>
  <footer class="border-t border-neutral-200 dark:border-neutral-800">
    <div class="mx-auto max-w-5xl px-6 py-12">
      <div class="grid grid-cols-2 gap-8 sm:grid-cols-4">
        <!-- Brand + tagline -->
        <div class="col-span-2 sm:col-span-1">
          <NuxtLink to="/" class="text-base font-semibold text-neutral-900 dark:text-neutral-100">{{ productName }}</NuxtLink>
          <p class="mt-2 max-w-xs text-sm leading-relaxed text-neutral-500 dark:text-neutral-400">
            Real-time docs, offline editing, and your own AI — one place to write.
          </p>
        </div>

        <!-- Product -->
        <div>
          <p class="text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Product</p>
          <div class="mt-3 flex flex-col gap-2 text-sm">
            <NuxtLink to="/#features" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Features</NuxtLink>
            <NuxtLink to="/changelog" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Changelog</NuxtLink>
          </div>
        </div>

        <!-- Account -->
        <div>
          <p class="text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Account</p>
          <div v-if="authReady && authStore.isAuthenticated" class="mt-3 flex flex-col gap-2 text-sm">
            <NuxtLink to="/app" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Open App</NuxtLink>
          </div>
          <div v-else class="mt-3 flex flex-col gap-2 text-sm">
            <NuxtLink to="/login" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Log in</NuxtLink>
            <NuxtLink to="/register" class="text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100">Sign up</NuxtLink>
          </div>
        </div>

        <!-- Built with -->
        <div>
          <p class="text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500">Built with</p>
          <p class="mt-3 text-sm leading-relaxed text-neutral-500 dark:text-neutral-400">
            Rust, Nuxt, PostgreSQL, Redis, Yjs, and Tiptap.
          </p>
        </div>
      </div>
    </div>

    <div class="border-t border-neutral-200 dark:border-neutral-800">
      <div class="mx-auto flex max-w-5xl items-center justify-between px-6 py-6">
        <p class="text-sm text-neutral-500 dark:text-neutral-400">
          &copy; {{ new Date().getFullYear() }} {{ productName }}. All rights reserved.
        </p>
        <NuxtLink to="/" class="text-xs text-neutral-400 transition-colors hover:text-neutral-600 dark:text-neutral-500 dark:hover:text-neutral-300">
          Back to top ↑
        </NuxtLink>
      </div>
    </div>
  </footer>
</template>
