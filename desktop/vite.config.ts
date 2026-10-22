import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Relative asset URLs keep the build independent of where the webview serves it from.
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
