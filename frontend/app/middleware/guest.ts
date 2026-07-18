// Opposite of middleware/auth.ts: keeps already-authenticated users off the
// auth pages (login/register/otp), bouncing them to the dashboard instead.
export default defineNuxtRouteMiddleware(async () => {
  const authStore = useAuthStore()

  if (!authStore.isAuthenticated) {
    try {
      // Captured here (top of the middleware, before any await) so it runs
      // in a guaranteed-valid Nuxt SSR context — there's no browser cookie
      // jar server-side, so the incoming request's cookie must be forwarded
      // by hand for the session check to work during SSR (otherwise it only
      // resolves client-side, after the page has already rendered logged-out).
      const headers = import.meta.server ? useRequestHeaders(['cookie']) : undefined
      await authStore.refresh(headers)
    } catch {
      return
    }
  }

  if (authStore.isAuthenticated) return navigateTo('/app')
})
