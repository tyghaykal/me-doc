<script setup lang="ts">
definePageMeta({ middleware: ['guest'] })

const auth = useAuthStore()

// guest middleware already resolved auth state (and would have redirected an
// authenticated user away) before this page rendered, so LandingNav can show
// its logged-out state immediately.
const authReady = useAuthReady()
onMounted(() => (authReady.value = true))

const { data: googleEnabled } = await useGoogleAuthEnabled()

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
    await auth.login(email.value, password.value)
    await navigateTo({ path: '/login-otp', query: { email: email.value } })
  } catch (err: any) {
    error.value = err?.data?.message ?? err?.message ?? 'Login failed.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="flex min-h-screen flex-col bg-white font-sans dark:bg-neutral-950">
    <LandingNav />

    <main class="flex flex-1 items-center justify-center p-8">
      <form
        class="w-full max-w-sm rounded-lg border border-neutral-200 bg-white p-8 shadow-xl dark:border-neutral-800 dark:bg-neutral-900"
        @submit.prevent="submit"
      >
        <h1 class="text-3xl font-bold text-neutral-900 dark:text-neutral-100">Log in</h1>
        <p class="mt-2 text-neutral-600 dark:text-neutral-400">Welcome back. Enter your credentials.</p>

        <template v-if="googleEnabled">
          <GoogleButton class="mt-6" />

          <div class="mt-4 flex items-center gap-3">
            <span class="h-px flex-1 bg-neutral-200 dark:bg-neutral-700" />
            <span class="text-xs text-neutral-500 dark:text-neutral-400">or</span>
            <span class="h-px flex-1 bg-neutral-200 dark:bg-neutral-700" />
          </div>
        </template>

        <label for="login-email" class="block mt-6 text-sm font-medium text-neutral-700 dark:text-neutral-300">Email</label>
        <input
          id="login-email"
          v-model="email"
          type="email"
          required
          autocomplete="email"
          class="mt-1 w-full rounded border border-neutral-300 px-3 py-2 dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
        />

        <label for="login-password" class="block mt-4 text-sm font-medium text-neutral-700 dark:text-neutral-300">Password</label>
        <input
          id="login-password"
          v-model="password"
          type="password"
          required
          minlength="8"
          autocomplete="current-password"
          class="mt-1 w-full rounded border border-neutral-300 px-3 py-2 dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
        />

        <p v-if="error" class="mt-4 text-sm text-red-600 dark:text-red-400">{{ error }}</p>

        <button
          type="submit"
          :disabled="loading"
          class="mt-6 w-full rounded bg-teal-600 px-4 py-2 font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
        >
          {{ loading ? 'Logging in…' : 'Log in' }}
        </button>

        <p class="mt-4 text-sm text-neutral-600 dark:text-neutral-400">
          No account?
          <NuxtLink to="/register" class="font-medium text-teal-700 underline dark:text-teal-400">Sign up</NuxtLink>
        </p>
      </form>
    </main>

    <LandingFooter />
  </div>
</template>
