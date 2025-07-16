import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  server: {
    host: true, // listen on all addresses
    port: 5173,
  },
  plugins: [vue()],
})
