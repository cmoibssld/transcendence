import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/auth': { target: 'https://localhost:8443', changeOrigin: true, secure: false },
      '/api':  { target: 'https://localhost:8443', changeOrigin: true, secure: false },
      '/ws':   { target: 'wss://localhost:8443',   ws: true, changeOrigin: true, secure: false },
    },
  },
})
