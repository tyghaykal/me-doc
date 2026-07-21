<script setup lang="ts">
const authStore = useAuthStore()
const authReady = useAuthReady()
</script>

<template>
  <section class="mx-auto max-w-4xl px-6 pb-20 pt-16 text-center sm:pb-28 sm:pt-24">
    <h1 class="text-4xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100 sm:text-6xl">
      One place for your team's <span class="underline decoration-2 underline-offset-4">notes and docs</span>
    </h1>
    <p class="mx-auto mt-6 max-w-2xl text-lg text-neutral-600 dark:text-neutral-400">
      Write together in real time, organize pages your way, share exactly what you mean to,
      and export whenever you need to take it elsewhere.
    </p>

    <div class="mt-10 flex items-center justify-center gap-4">
      <div v-if="!authReady" class="invisible h-11 w-[280px]" />
      <template v-else-if="authStore.isAuthenticated">
        <NuxtLink
          to="/app"
          class="rounded-md bg-neutral-900 px-5 py-2.5 text-sm font-semibold text-white hover:bg-neutral-700 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
        >
          Open App
        </NuxtLink>
      </template>
      <template v-else>
        <NuxtLink
          to="/register"
          class="rounded-md bg-neutral-900 px-5 py-2.5 text-sm font-semibold text-white hover:bg-neutral-700 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
        >
          Get started — it's free
        </NuxtLink>
        <NuxtLink
          to="/login"
          class="rounded-md border border-neutral-300 px-5 py-2.5 text-sm font-semibold text-neutral-700 hover:bg-neutral-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
        >
          Log in
        </NuxtLink>
      </template>
    </div>
  </section>
</template>
