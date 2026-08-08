import { defineConfig } from 'vite';
import { resolve } from 'path';

// Multi-page build: one webview window per page (main panel / settings / tray menu)
export default defineConfig({
  base: './',
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Cargo build outputs are locked/rewritten while rustc runs
      ignored: ['**/src-tauri/target/**'],
    },
  },
  build: {
    target: 'es2021',
    // Inline small assets (the logo) as data URIs so no asset:// protocol is needed
    assetsInlineLimit: 100 * 1024,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        settings: resolve(__dirname, 'settings.html'),
        menu: resolve(__dirname, 'menu.html'),
      },
    },
  },
});
