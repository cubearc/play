import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  root: "desktop-renderer-src",
  base: "./",
  plugins: [react()],
  build: {
    outDir: "../desktop-renderer",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: "app.js",
        assetFileNames: "app.[ext]",
      },
    },
  },
});
