<script setup lang="ts">
definePageMeta({ middleware: ['guest'] })

const auth = useAuthStore()

const email = ref('')
const password = ref('')
const error = ref<string | null>(null)
const loading = ref(false)

async function submit() {
  error.value = null
  if (password.value.length < 8) {
    error.value = 'Password must be at least 8 characters.'
    return
  }
  loading.value = true
  try {
    await auth.register(email.value, password.value)
    await navigateTo({ path: '/verify-otp', query: { email: email.value } })
  } catch (err: any) {
    error.value = err?.data?.message ?? err?.message ?? 'Registration failed.'
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
      <h1 class="text-3xl font-bold text-slate-900 dark:text-slate-100">Create account</h1>
      <p class="mt-2 text-slate-600 dark:text-slate-400">Sign up with your email and a password.</p>

      <label class="block mt-6 text-sm font-medium text-slate-700 dark:text-slate-300">Email</label>
      <input
        v-model="email"
        type="email"
        required
        autocomplete="email"
        class="mt-1 w-full rounded border border-slate-300 px-3 py-2 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
      />

      <label class="block mt-4 text-sm font-medium text-slate-700 dark:text-slate-300">Password</label>
      <input
        v-model="password"
        type="password"
        required
        minlength="8"
        autocomplete="new-password"
        class="mt-1 w-full rounded border border-slate-300 px-3 py-2 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
      />

      <p v-if="error" class="mt-4 text-sm text-red-600 dark:text-red-400">{{ error }}</p>

      <button
        type="submit"
        :disabled="loading"
        class="mt-6 w-full rounded bg-slate-900 px-4 py-2 font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
      >
        {{ loading ? 'Creating…' : 'Create account' }}
      </button>

      <p class="mt-4 text-sm text-slate-600 dark:text-slate-400">
        Already have an account?
        <NuxtLink to="/login" class="font-medium text-slate-900 underline dark:text-slate-100">Log in</NuxtLink>
      </p>
    </form>
  </main>
</template>
