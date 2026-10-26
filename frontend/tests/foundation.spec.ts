import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("connects the real UI to the local Rust server and survives refresh", async ({
  page,
}) => {
  const sessionUrl = process.env.CUBE_PLAY_TEST_URL;
  if (!sessionUrl) throw new Error("Local session was not initialized");
  const origin = new URL(sessionUrl).origin;
  const errors: string[] = [];
  const externalRequests: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (new URL(request.url()).origin !== origin)
      externalRequests.push(request.url());
  });
  await page.goto(`${sessionUrl}&file=sample+%E7%A9%BA.mp4&intent=file`);
  await expect(
    page.getByRole("heading", { name: "Your connection is ready" }),
  ).toBeVisible();
  await expect(page).toHaveURL(`${origin}/`);
  await expect(page.getByText("Connected locally")).toBeVisible();
  const health = await (await page.request.get("/api/health")).json();
  await expect(
    page.getByRole("heading", { name: health.workspace.name, exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Opened file:")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "sample 空.mp4", exact: true }),
  ).toBeVisible();
  const cookie = (await page.context().cookies()).find(
    (value) => value.name === `cube_play_session_${new URL(sessionUrl).port}`,
  );
  expect(cookie?.httpOnly).toBe(true);
  expect(cookie?.sameSite).toBe("Strict");
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Your connection is ready" }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  expect(errors).toEqual([]);
  expect(externalRequests).toEqual([]);
  const accessibility = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(accessibility.violations).toEqual([]);
});

test("shows session recovery rather than pretending an unauthorized client is connected", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Try again" })).toBeVisible();
  await expect(page.getByRole("status")).toContainText("Open the session link");
  await expect(page.getByText("Connected locally")).toHaveCount(0);
});

test("rejects unauthorized and cross-origin API access", async ({
  request,
}) => {
  expect((await request.get("/api/health")).status()).toBe(401);
  expect(
    (
      await request.get("/healthz", {
        headers: { Origin: "https://attacker.example" },
      })
    ).status(),
  ).toBe(403);
  expect(
    (
      await request.get("/healthz", {
        headers: { Host: "attacker.example:17331" },
      })
    ).status(),
  ).toBe(403);
});
