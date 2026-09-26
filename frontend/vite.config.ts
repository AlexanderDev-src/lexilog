import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  server: {
    // This machine only, like the app itself. To try it on a phone, use
    // `tailscale serve` for port 5173 instead of opening it to the LAN.
    host: '127.0.0.1',
    // The Rust backend runs on 1111; forward API calls to it.
    proxy: {
      '/api': 'http://localhost:1111',
    },
  },
})
