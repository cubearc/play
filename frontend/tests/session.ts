import { type APIRequestContext, test as base } from "@playwright/test";

export { expect } from "@playwright/test";

// API assertions use a separate client. Browser requests must prove that the
// application itself supplies authorization; no test header is injected there.
export const test = base.extend<{ api: APIRequestContext }>({
  api: async ({ playwright }, use) => {
    const link = process.env.CUBE_PLAY_TEST_URL;
    if (!link) throw new Error("The local test session is unavailable.");
    const url = new URL(link);
    const token = new URLSearchParams(url.hash.slice(1)).get("session");
    if (!token) throw new Error("The local test capability is unavailable.");
    const client = await playwright.request.newContext({
      baseURL: url.origin,
      extraHTTPHeaders: { Authorization: `Bearer ${token}` },
    });
    try {
      await use(client);
    } finally {
      await client.dispose();
    }
  },
});
