import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// `tauri android dev` sets this: the phone loads the dev server over the network, not from localhost.
const host = process.env.TAURI_DEV_HOST;

// Relative asset URLs keep the build independent of where the webview serves it from.
export default defineConfig({
  base: './',
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 5174 } : undefined,
    // The texts are shared by every program, in `locales/` next to `desktop/`.
    fs: { allow: ['..'] },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
