<script setup lang="ts">
// nuxt.config.ts's `app.head.title` is frozen at build time; PRODUCT_NAME is a
// docker-compose *runtime* env var (like NUXT_PUBLIC_API_BASE), so the title
// has to be set here via runtimeConfig (resolved per-request/on hydration)
// for a white-label operator's name to show up without rebuilding the image.
const { public: { productName } } = useRuntimeConfig()
useHead({ title: `${productName} — Write together, live!` })
</script>

<template>
  <div>
    <NuxtRouteAnnouncer />
    <NuxtPage />
    <AiStatusToast />
  </div>
</template>
