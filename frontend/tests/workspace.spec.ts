import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "./session";

test.beforeEach(async ({ page, api }) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  const token = new URLSearchParams(new URL(url).hash.slice(1)).get("session");
  await api.post("/api/session", {
    headers: {
      Origin: new URL(url).origin,
      Authorization: `Bearer ${token}`,
    },
  });
  const snapshot = await (await api.get("/api/investigation")).json();
  const reset = await api.put("/api/investigation/view", {
    headers: { Origin: new URL(url).origin },
    data: {
      revision: snapshot.view_revision,
      view: { store: "source", path: "", offset: 0, selected: null },
    },
  });
  expect(reset.ok()).toBe(true);
  await page.goto(url);
  await expect(
    page.getByRole("button", { name: "Inspect sample.png", exact: true }),
  ).toBeVisible();
});

test("browses real files and renders Core-backed previews with inert fallback content", async ({
  page,
}) => {
  const external: string[] = [];
  page.on("request", (request) => {
    if (
      new URL(request.url()).origin !==
      new URL(process.env.CUBE_PLAY_TEST_URL ?? "").origin
    )
      external.push(request.url());
  });
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  const image = page.getByRole("img", { name: "Preview of sample.png" });
  await expect(image).toBeVisible();
  await expect
    .poll(() =>
      image.evaluate(
        (image) => image instanceof HTMLImageElement && image.naturalWidth,
      ),
    )
    .toBe(2);
  await expect(page.getByText("2 × 2 pixels")).toBeVisible();
  await page
    .getByRole("button", { name: "Inspect notes.txt", exact: true })
    .click();
  await expect(
    page.getByRole("region", { name: "Text preview" }),
  ).toContainText("<script>window.fixtureExecuted = true</script>");
  expect(await page.evaluate(() => "fixtureExecuted" in window)).toBe(false);
  await page
    .getByRole("button", { name: "Inspect unknown.xyz", exact: true })
    .click();
  await expect(
    page.getByRole("region", { name: "Byte inspection" }),
  ).toContainText("00ff112a");
  await page
    .getByRole("button", { name: "Inspect sample 空.mp4", exact: true })
    .click();
  await expect(
    page.getByText(/outside the supported Cube metadata profiles/),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Inspect sample.png", exact: true }),
  ).toBeVisible();
  expect(external).toEqual([]);
});

test("auto-opens a single compatible file and handles an empty directory", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Open directory nested", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "single.txt", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Text preview" }),
  ).toContainText("One compatible file");
  await page.getByRole("button", { name: "Up one directory" }).click();
  await page
    .getByRole("button", { name: "Open directory empty", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "This workspace is empty" }),
  ).toBeVisible();
  await expect(page.getByLabel("Choose a file")).toBeEnabled();
});

test("serves real video/audio ranges and preserves a private explicit import after refresh", async ({
  page,
  api,
}) => {
  await page
    .getByRole("button", { name: "Inspect sample.mp4", exact: true })
    .click();
  const video = page.getByLabel("Video preview of sample.mp4");
  await expect
    .poll(
      async () =>
        (await video.isVisible()) ||
        (await page
          .getByText(
            "The browser could not decode this media. Its metadata and byte inspection remain available below.",
            { exact: true },
          )
          .isVisible()),
    )
    .toBe(true);
  await expect(page.getByText("Cube container metadata")).toBeVisible();
  const response = await api.get("/api/workspace/media?path=sample.mp4", {
    headers: { Range: "bytes=0-15" },
  });
  expect(response.status()).toBe(206);
  expect((await response.body()).length).toBe(16);
  await page
    .getByRole("button", { name: "Inspect sample.wav", exact: true })
    .click();
  await expect(page.getByLabel("Audio preview of sample.wav")).toBeVisible();
  await page.getByLabel("Choose a file").setInputFiles({
    name: `private-${test.info().project.name}.txt`,
    mimeType: "text/plain",
    buffer: Buffer.from("Explicit import; stays local."),
  });
  await page
    .getByRole("button", { name: "Import into workspace", exact: true })
    .click();
  await expect(
    page.getByRole("region", { name: "Text preview" }),
  ).toContainText("Explicit import; stays local.");
  await page.reload();
  await page
    .getByRole("button", { name: "Imported files", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: `Inspect private-${test.info().project.name}.txt`,
      exact: true,
    }),
  ).toBeVisible();
  const accessibility = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(accessibility.violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("restores investigation name and file selection across browser reload", async ({
  page,
}) => {
  const name = `Case ${test.info().project.name}`;
  await page.getByLabel("Investigation name").fill(name);
  const renamed = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/investigation") &&
      response.request().method() === "PUT",
  );
  await page.getByRole("button", { name: "Save name", exact: true }).click();
  expect((await renamed).ok()).toBe(true);
  const saved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/investigation/view") &&
      response.request().method() === "PUT",
  );
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  expect((await saved).ok()).toBe(true);
  await page.reload();
  await expect(page.getByLabel("Investigation name")).toHaveValue(name);
  await expect(
    page.getByRole("img", { name: "Preview of sample.png" }),
  ).toBeVisible();
});

test("prevents another tab from silently replacing saved navigation", async ({
  page,
  context,
}) => {
  const second = await context.newPage();
  await second.goto("/");
  await expect(second.getByRole("button", { name: "Try again" })).toBeVisible();
  await second.goto(process.env.CUBE_PLAY_TEST_URL ?? "");
  await expect(
    second.getByRole("button", { name: "Inspect sample.png", exact: true }),
  ).toBeVisible();
  const firstSaved = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/investigation/view") &&
      response.request().method() === "PUT",
  );
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  expect((await firstSaved).ok()).toBe(true);
  await second
    .getByRole("button", { name: "Inspect notes.txt", exact: true })
    .click();
  await expect(second.getByRole("alert")).toContainText(
    "changed in another request",
  );
  await second.getByRole("button", { name: "Reload investigation" }).click();
  await expect(
    second.getByRole("img", { name: "Preview of sample.png" }),
  ).toBeVisible();
  await second.close();
});
