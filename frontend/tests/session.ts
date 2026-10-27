import {
  type APIRequestContext,
  test as base,
  expect,
  type Page,
} from "@playwright/test";

export { expect } from "@playwright/test";

export async function openSection(page: Page, name: string) {
  await page.getByRole("tab", { name, exact: true }).click();
  await expect(page.getByRole("tabpanel", { name, exact: true })).toBeVisible();
  await expect(
    page
      .getByRole("region", { name: "Saved investigation", exact: true })
      .getByRole("status"),
  ).toHaveText("Navigation saved locally");
}
export async function openQuickTools(page: Page) {
  await openSection(page, "Explorations");
  if (
    !(await page
      .getByRole("region", { name: "Text and byte Lab", exact: true })
      .isVisible())
  )
    await page.getByText("Quick text and byte tools", { exact: true }).click();
}

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
