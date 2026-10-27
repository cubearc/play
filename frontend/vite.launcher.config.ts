import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "launcher-dist",
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: {
      entry: "src/launcher-main.tsx",
      name: "CubePlayLauncher",
      formats: ["iife"],
      fileName: () => "launcher.js",
      cssFileName: "launcher",
    },
  },
});
