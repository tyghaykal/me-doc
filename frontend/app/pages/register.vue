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
  <main class="min-h-screen flex items-center justify-center p-8 font-sans bg-neutral-50 dark:bg-neutral-950">
    <form
      class="w-full max-w-sm rounded-lg border border-neutral-200 bg-white p-8 shadow-xl dark:border-neutral-800 dark:bg-neutral-900"
      @submit.prevent="submit"
    >
      <h1 class="text-3xl font-bold text-neutral-900 dark:text-neutral-100">Create account</h1>
      <p class="mt-2 text-neutral-600 dark:text-neutral-400">Sign up with your email and a password.</p>

      <label class="block mt-6 text-sm font-medium text-neutral-700 dark:text-neutral-300">Email</label>
      <input
        v-model="email"
        type="email"
        required
        autocomplete="email"
        class="mt-1 w-full rounded border border-neutral-300 px-3 py-2 dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
      />

      <label class="block mt-4 text-sm font-medium text-neutral-700 dark:text-neutral-300">Password</label>
      <input
        v-model="password"
        type="password"
        required
        minlength="8"
        autocomplete="new-password"
        class="mt-1 w-full rounded border border-neutral-300 px-3 py-2 dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
      />

      <p v-if="error" class="mt-4 text-sm text-red-600 dark:text-red-400">{{ error }}</p>

      <button
        type="submit"
        :disabled="loading"
        class="mt-6 w-full rounded bg-teal-600 px-4 py-2 font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
      >
        {{ loading ? 'Creating…' : 'Create account' }}
      </button>

      <p class="mt-4 text-sm text-neutral-600 dark:text-neutral-400">
        Already have an account?
        <NuxtLink to="/login" class="font-medium text-teal-700 underline dark:text-teal-400">Log in</NuxtLink>
      </p>
    </form>
  </main>
</template>
