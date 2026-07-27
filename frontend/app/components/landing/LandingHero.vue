<script setup lang="ts">
const authStore = useAuthStore()
const authReady = useAuthReady()
</script>

<template>
  <section class="mx-auto max-w-4xl px-6 pb-20 pt-16 text-center sm:pb-28 sm:pt-24">
    <h1 class="animate-fade-up text-4xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100 sm:text-6xl">
      One place for your team's <span class="underline decoration-teal-500 decoration-2 underline-offset-4 dark:decoration-teal-400">notes and docs</span>
    </h1>
    <p class="animate-fade-up mx-auto mt-6 max-w-2xl text-lg text-neutral-600 dark:text-neutral-400 [animation-delay:80ms]">
      Write together in real time, organize pages your way, share exactly what you mean to,
      and export whenever you need to take it elsewhere.
    </p>

    <div class="animate-fade-up mt-10 flex items-center justify-center gap-4 [animation-delay:160ms]">
      <div v-if="!authReady" class="invisible h-11 w-[280px]" />
      <template v-else-if="authStore.isAuthenticated">
        <NuxtLink
          to="/app"
          class="rounded-md bg-teal-600 px-5 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-teal-700 active:scale-[0.98] dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
        >
          Open App
        </NuxtLink>
      </template>
      <template v-else>
        <NuxtLink
          to="/register"
          class="rounded-md bg-teal-600 px-5 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-teal-700 active:scale-[0.98] dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
        >
          Get started — it's free
        </NuxtLink>
        <NuxtLink
          to="/login"
          class="rounded-md border border-neutral-300 px-5 py-2.5 text-sm font-semibold text-neutral-700 transition-colors hover:bg-neutral-50 active:scale-[0.98] dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
        >
          Log in
        </NuxtLink>
      </template>
    </div>
  </section>
</template>
