<script setup lang="ts">
// No auth middleware — the changelog is public, same as the landing page,
// but shows the authenticated app shell (sidebar) instead of the marketing
// header/footer once we know a session exists.

const authStore = useAuthStore()
const authReady = useAuthReady()
useAppShellData()

onMounted(async () => {
  if (!authStore.isAuthenticated) {
    try {
      await authStore.refresh()
    } catch {
      // not logged in — stay on the public changelog as a guest
    }
  }
  authReady.value = true
})
</script>

<template>
  <div v-if="!authReady" class="min-h-screen bg-white dark:bg-neutral-950" />

  <div v-else-if="authStore.isAuthenticated && authStore.workspace" class="flex h-screen font-sans">
    <AppSidebar :workspace-id="authStore.workspace.id" />

    <div class="flex min-w-0 flex-1 flex-col bg-white dark:bg-neutral-900">
      <AppTopbar :active-page="null" />

      <main class="min-h-0 min-w-0 flex-1 overflow-y-auto thin-scrollbar p-4 pt-14 sm:p-8 sm:pt-8">
        <ChangelogEntries />
      </main>
    </div>
  </div>

  <div v-else class="flex min-h-screen flex-col bg-white font-sans dark:bg-neutral-950">
    <LandingNav />

    <main class="mx-auto w-full flex-1 px-6 py-12">
      <ChangelogEntries />
    </main>

    <LandingFooter />
  </div>
</template>
