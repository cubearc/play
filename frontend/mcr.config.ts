import type { CoverageReportOptions } from "monocart-coverage-reports";
import { rendererCoverage } from "./coverage.config.ts";

export default {
  ...rendererCoverage,
  outputDir: "coverage/unit",
  reports: ["raw", "json-summary", "console-summary"],
} satisfies CoverageReportOptions;
