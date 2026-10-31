import { resolve } from "node:path";
import type { CoverageReportOptions } from "monocart-coverage-reports";

// One source identity and one V8 converter for unit and actual Electron profiles.
export const rendererCoverage = {
  entryFilter: (entry: { url: string }) =>
    entry.url.includes("desktop-renderer-src/") ||
    entry.url.endsWith("/app.js"),
  sourceFilter: (path: string) =>
    path.includes("desktop-renderer-src/") &&
    !path.includes("/test/") &&
    !path.includes(".test."),
  sourcePath: (path: string) => {
    const index = path.lastIndexOf("desktop-renderer-src/");
    return index < 0 ? path : resolve(process.cwd(), path.slice(index));
  },
} satisfies CoverageReportOptions;
