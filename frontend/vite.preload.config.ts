import { defineConfig } from "vite";

export default defineConfig({
  build: {
    outDir: "desktop-dist",
    emptyOutDir: false,
    lib: {
      entry: "desktop/preload.ts",
      formats: ["cjs"],
      fileName: () => "preload.cjs",
    },
    rollupOptions: { external: ["electron"] },
  },
});
