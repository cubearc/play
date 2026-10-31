import { defineConfig } from "vite";

export default defineConfig({
  build: {
    ssr: "desktop/main.ts",
    target: "node24",
    outDir: "desktop-main",
    emptyOutDir: true,
    rollupOptions: {
      external: (id) => id === "electron" || id.startsWith("node:"),
      output: { entryFileNames: "main.js", format: "es" },
    },
  },
});
