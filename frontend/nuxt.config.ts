import tailwindcss from '@tailwindcss/vite'

// White-label name — operators override via NUXT_PUBLIC_PRODUCT_NAME (build-time,
// baked into the title below) and PRODUCT_NAME on the backend (see config.rs).
const productName = process.env.NUXT_PUBLIC_PRODUCT_NAME || 'MeDoc'

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },
  modules: ['@pinia/nuxt'],
  css: ['~/assets/css/main.css'],
  app: {
    head: {
      // Dark is the default theme, rendered server-side so there's no flash.
      // The only case needing client JS is "user previously chose light" —
      // remove the class before paint (blocking script, default head position).
      htmlAttrs: { class: 'dark' },
      title: `${productName} — Write together, live!`,
      titleTemplate: '%s',
      meta: [
        { name: 'description', content: 'Real-time collaborative notes and docs for your team.' },
      ],
      link: [
        { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
        { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png' },
        { rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png' },
      ],
      script: [
        {
          innerHTML: `(function(){try{if(localStorage.getItem('theme')==='light')document.documentElement.classList.remove('dark')}catch(e){}})()`,
        },
      ],
    },
  },
  vite: {
    plugins: [tailwindcss()],
  },
  routeRules: {
    // Authenticated app shell: client-rendered only, calls the Rust API directly.
    '/app/**': { ssr: false },
  },
  runtimeConfig: {
    // Server-only: used for SSR fetches made from inside the frontend container,
    // where "localhost" would resolve to the frontend container itself, not the backend.
    apiBaseServer: process.env.NUXT_API_BASE_SERVER || 'http://backend:8080',
    public: {
      // Client-facing: used for browser fetches, which reach the backend via the host machine.
      apiBase: process.env.NUXT_PUBLIC_API_BASE || 'http://localhost:8080',
      // Browser-reachable MinIO bucket base for reading uploaded attachments (public download).
      // The backend signs against the internal docker host (minio:9000); this is the host mapping.
      minioBase: process.env.NUXT_PUBLIC_MINIO_BASE || 'http://localhost:9010/medoc',
      productName,
    },
  },
})
