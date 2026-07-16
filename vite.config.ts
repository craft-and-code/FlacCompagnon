import { defineConfig } from "vite";

// Vite config tuned for Tauri: fixed dev port, no clearing of the terminal so
// Rust build output stays visible.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Don't watch the Rust side from the frontend dev server.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "esnext",
    outDir: "dist",
  },
});
