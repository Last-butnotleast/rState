import { fileURLToPath, URL } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@reference/store": fileURLToPath(
        new URL("./.reference/src/store.ts", import.meta.url),
      ),
      "@reference/atom": fileURLToPath(
        new URL("./.reference/src/atom.ts", import.meta.url),
      ),
      "@reference/reset": fileURLToPath(
        new URL("./.reference/src/reset.ts", import.meta.url),
      ),
      "@reference/undo": fileURLToPath(
        new URL("./.reference/src/undo.ts", import.meta.url),
      ),
      "@reference/persist": fileURLToPath(
        new URL("./.reference/src/persist.ts", import.meta.url),
      ),
    },
  },
  server: { host: "127.0.0.1", port: 4173, strictPort: true },
});
