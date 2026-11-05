import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Relative asset URLs keep the build independent of where the webview serves it from.
export default defineConfig({
  base: './',
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    // The texts are one file for every program, `texts/texts.json` next to `desktop/`.
    fs: { allow: ['..'] },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
