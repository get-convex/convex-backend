// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-05-15',
  devtools: { enabled: true },
  modules: ['convex-nuxt'],
  convex: {
    url: process.env.CONVEX_URL,
  }
})