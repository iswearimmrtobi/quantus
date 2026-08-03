import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// The packaged app loads the build from file://, so every asset URL has to be relative.
export default defineConfig({
  base: './',
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
