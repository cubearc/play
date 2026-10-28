import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  root: "desktop-renderer-src",
  base: "./",
  plugins: [
    react(),
    {
      name: "packaged-codicon-font",
      enforce: "pre",
      transform(code, id) {
        if (id.includes("/@vscode/codicons/dist/codicon.css")) {
          // Packaged resources are uncached and use exact names, without URL queries.
          return code.replace(/codicon\.ttf\?[a-f0-9]+/g, "codicon.ttf");
        }
        return undefined;
      },
    },
  ],
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
