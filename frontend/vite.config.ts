import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  server: {
    // Reachable from other devices on the home network during development.
    host: true,
    // The Rust backend runs on 1111; forward API calls to it.
    proxy: {
      '/api': 'http://localhost:1111',
    },
  },
})
