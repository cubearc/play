import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./desktop-renderer-src/test/setup.ts"],
    include: ["desktop-renderer-src/**/*.test.{ts,tsx}"],
    clearMocks: true,
    restoreMocks: true,
    coverage: {
      provider: "custom",
      customProviderModule: "vitest-monocart-coverage",
      reportsDirectory: "coverage/unit",
      include: ["desktop-renderer-src/**/*.{ts,tsx}"],
      exclude: [
        "desktop-renderer-src/test/**",
        "desktop-renderer-src/**/*.test.{ts,tsx}",
      ],
      // Unit and real Electron coverage share the unchanged gates in scripts/coverage.mjs.
    },
  },
});
