<script setup lang="ts">
definePageMeta({ middleware: ['guest'] })

const auth = useAuthStore()
const route = useRoute()

const email = String(route.query.email ?? '')
const code = ref('')
const error = ref<string | null>(null)
const loading = ref(false)

async function submit() {
  error.value = null
  if (!/^\d{6}$/.test(code.value)) {
    error.value = 'Enter the 6-digit code.'
    return
  }
  loading.value = true
  try {
    await auth.verifyLogin(email, code.value)
    await navigateTo('/app')
  } catch (err: any) {
    error.value = err?.data?.message ?? err?.message ?? 'Verification failed.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="min-h-screen flex items-center justify-center p-8 font-sans bg-neutral-50 dark:bg-neutral-950">
    <form
      class="w-full max-w-sm rounded-lg border border-neutral-200 bg-white p-8 shadow-xl dark:border-neutral-800 dark:bg-neutral-900"
      @submit.prevent="submit"
    >
      <h1 class="text-3xl font-bold text-neutral-900 dark:text-neutral-100">Confirm login</h1>
      <p class="mt-2 text-neutral-600 dark:text-neutral-400">
        Enter the 6-digit code sent to
        <span class="font-medium text-neutral-900 dark:text-neutral-100">{{ email || 'your email' }}</span>.
      </p>

      <label class="block mt-6 text-sm font-medium text-neutral-700 dark:text-neutral-300">Code</label>
      <input
        v-model="code"
        inputmode="numeric"
        maxlength="6"
        required
        autocomplete="one-time-code"
        class="mt-1 w-full rounded border border-neutral-300 px-3 py-2 tracking-widest dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
      />

      <p v-if="error" class="mt-4 text-sm text-red-600 dark:text-red-400">{{ error }}</p>

      <button
        type="submit"
        :disabled="loading"
        class="mt-6 w-full rounded bg-teal-600 px-4 py-2 font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
      >
        {{ loading ? 'Verifying…' : 'Verify' }}
      </button>
    </form>
  </main>
</template>
