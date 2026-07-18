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
  <main class="min-h-screen flex items-center justify-center p-8 font-sans bg-slate-50 dark:bg-slate-950">
    <form
      class="w-full max-w-sm rounded-lg border border-slate-200 bg-white p-8 shadow-xl dark:border-slate-800 dark:bg-slate-900"
      @submit.prevent="submit"
    >
      <h1 class="text-3xl font-bold text-slate-900 dark:text-slate-100">Confirm login</h1>
      <p class="mt-2 text-slate-600 dark:text-slate-400">
        Enter the 6-digit code sent to
        <span class="font-medium text-slate-900 dark:text-slate-100">{{ email || 'your email' }}</span>.
      </p>

      <label class="block mt-6 text-sm font-medium text-slate-700 dark:text-slate-300">Code</label>
      <input
        v-model="code"
        inputmode="numeric"
        maxlength="6"
        required
        autocomplete="one-time-code"
        class="mt-1 w-full rounded border border-slate-300 px-3 py-2 tracking-widest dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
      />

      <p v-if="error" class="mt-4 text-sm text-red-600 dark:text-red-400">{{ error }}</p>

      <button
        type="submit"
        :disabled="loading"
        class="mt-6 w-full rounded bg-slate-900 px-4 py-2 font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
      >
        {{ loading ? 'Verifying…' : 'Verify' }}
      </button>
    </form>
  </main>
</template>
