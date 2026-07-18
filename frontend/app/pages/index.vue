<script setup lang="ts">
// Silent session check so the nav/hero can offer "Open App" instead of
// Login/Sign up when the visitor already has a valid session (accessToken is
// in-memory only, so a fresh load needs this to recover it from the
// httpOnly refresh cookie).
const authStore = useAuthStore()
const authReady = useAuthReady()

onMounted(async () => {
  if (!authStore.isAuthenticated) {
    try {
      await authStore.refresh()
    } catch {
      // not logged in — stay on the marketing page as a guest
    }
  }
  authReady.value = true
})
</script>

<template>
  <div class="min-h-screen bg-white font-sans dark:bg-slate-950">
    <LandingNav />
    <LandingHero />
    <LandingFeatures />
    <LandingFooter />
  </div>
</template>
