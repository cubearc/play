import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";
import { setSessionToken } from "../capability";

vi.mock("../capability", async (original) => ({
  ...(await original<typeof import("../capability")>()),
  readLink: vi.fn(
    async (uri: string) =>
      `${uri}${uri.includes("?") ? "&" : "?"}read_cap=6d434f94-89f3-4fb8-a7d8-c345a0000001`,
  ),
}));

afterEach(() => {
  cleanup();
  setSessionToken(null);
});
