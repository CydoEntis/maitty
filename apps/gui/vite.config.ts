import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(fileURLToPath(new URL('.', import.meta.url)), 'src') },
  },
  // The window loads this exact address (`devUrl` in tauri.conf.json). With strictPort a taken port is an
  // error, not a quiet move to another one, which would show the window some other project's dev server.
  server: { port: 1420, strictPort: true },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    restoreMocks: true,
    // The App-level tests render the whole window and take 5 to 7 s on CI's Windows and Arch
    // runners, past vitest's 5 s default. A slow render is not a failing test.
    testTimeout: 30_000,
  },
})
