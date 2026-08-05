<script setup lang="ts">
// End of the server-side Google OAuth round trip. The backend already issued
// the refresh_token cookie and redirected here; all that's left is to exchange
// that cookie for a session and enter the app. No `guest` middleware — an
// authenticated visitor must be allowed to reach this page to finish.
const auth = useAuthStore()
const error = ref<string | null>(null)

onMounted(async () => {
  try {
    await auth.refresh()
    await navigateTo('/app')
  } catch (err: any) {
    error.value = err?.data?.message ?? err?.message ?? 'Google sign-in failed.'
  }
})
</script>

<template>
  <main class="flex min-h-screen items-center justify-center p-8 font-sans bg-neutral-50 dark:bg-neutral-950">
    <div class="w-full max-w-sm rounded-lg border border-neutral-200 bg-white p-8 text-center shadow-xl dark:border-neutral-800 dark:bg-neutral-900">
      <p v-if="!error" class="text-neutral-600 dark:text-neutral-400">Signing you in…</p>
      <template v-else>
        <p class="text-red-600 dark:text-red-400">{{ error }}</p>
        <NuxtLink
          to="/login"
          class="mt-4 inline-block font-medium text-teal-700 underline dark:text-teal-400"
        >
          Back to login
        </NuxtLink>
      </template>
    </div>
  </main>
</template>
