export default defineNuxtRouteMiddleware(async () => {
  const authStore = useAuthStore()

  if (authStore.isAuthenticated) return

  try {
    await authStore.refresh()
  } catch {
    return navigateTo('/login')
  }

  if (!authStore.isAuthenticated) return navigateTo('/login')
})
