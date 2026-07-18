<script setup lang="ts">
const authStore = useAuthStore()
const authReady = useAuthReady()
</script>

<template>
  <section class="mx-auto max-w-4xl px-6 pb-20 pt-16 text-center sm:pb-28 sm:pt-24">
    <h1 class="text-4xl font-bold tracking-tight text-slate-900 dark:text-slate-100 sm:text-6xl">
      One place for your team's <span class="text-indigo-600 dark:text-indigo-400">notes and docs</span>
    </h1>
    <p class="mx-auto mt-6 max-w-2xl text-lg text-slate-600 dark:text-slate-400">
      Write together in real time, organize pages your way, share exactly what you mean to,
      and export whenever you need to take it elsewhere.
    </p>

    <div class="mt-10 flex items-center justify-center gap-4">
      <div v-if="!authReady" class="invisible h-11 w-[280px]" />
      <template v-else-if="authStore.isAuthenticated">
        <NuxtLink
          to="/app"
          class="rounded-md bg-indigo-600 px-5 py-2.5 text-sm font-semibold text-white hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400"
        >
          Open App
        </NuxtLink>
      </template>
      <template v-else>
        <NuxtLink
          to="/register"
          class="rounded-md bg-indigo-600 px-5 py-2.5 text-sm font-semibold text-white hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400"
        >
          Get started — it's free
        </NuxtLink>
        <NuxtLink
          to="/login"
          class="rounded-md border border-slate-300 px-5 py-2.5 text-sm font-semibold text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
        >
          Log in
        </NuxtLink>
      </template>
    </div>
  </section>
</template>
