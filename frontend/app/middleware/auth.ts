export default defineNuxtRouteMiddleware(async (to) => {
  const authStore = useAuthStore()
  const hasLink = typeof to.query.link === 'string'

  if (authStore.isAuthenticated) return

  try {
    await authStore.refresh()
  } catch {
    // A public share link grants access on its own (resolved server-side by
    // PagePermission) — anonymous visitors must not be bounced to /login.
    if (hasLink) return
    return navigateTo('/login')
  }

  if (!authStore.isAuthenticated && !hasLink) return navigateTo('/login')
})
