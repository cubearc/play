import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Session unavailable");
  await page.goto(url);
  await expect(
    page.getByRole("button", { name: "Source files", exact: true }),
  ).toBeVisible();
});
test("opens a launcher challenge, navigates authorized artifacts and restores private progress", async ({
  page,
}) => {
  const origin = new URL(process.env.CUBE_PLAY_TEST_URL ?? "").origin;
  const before = await (
    await page.request.get("/api/workspace/media?path=rabbit.cube")
  ).body();
  await page.goto(`${origin}/#file=rabbit.cube&intent=challenge`);
  // OS browser launching opens a document; a same-document test navigation does not.
  await page.reload();
  const panel = page.getByRole("region", {
    name: "Local challenges",
    exact: true,
  });
  await expect(
    panel.getByRole("heading", {
      name: "Local rabbit investigation",
      exact: true,
    }),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "Inspect evidence", exact: true })
    .click();
  await expect(panel.getByText(/window.challengeExecuted/)).toBeVisible();
  expect(await page.evaluate(() => "challengeExecuted" in window)).toBe(false);
  await panel.getByText("Hint 1", { exact: true }).click();
  await expect(
    panel.getByText("Try the green channel.", { exact: true }),
  ).toBeVisible();
  await panel
    .getByRole("combobox", { name: "Private local progress", exact: true })
    .selectOption("in_progress");
  await expect(
    panel.getByRole("combobox", {
      name: "Private local progress",
      exact: true,
    }),
  ).toHaveValue("in_progress");
  await panel
    .getByRole("button", {
      name: "Open challenge artifact sample",
      exact: true,
    })
    .click();
  await expect(
    page.getByRole("img", { name: "Preview of sample.png", exact: true }),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "Record a candidate", exact: true })
    .click();
  await expect(
    panel.getByText(
      "Select a private text result or an explicit structured field.",
      { exact: true },
    ),
  ).toBeVisible();
  await page.reload();
  await expect(
    panel.getByRole("button", { name: "Record a candidate", exact: true }),
  ).toHaveAttribute("aria-current", "step");
  await panel
    .getByRole("button", { name: "Inspect evidence", exact: true })
    .click();
  await expect(
    panel.getByRole("combobox", {
      name: "Private local progress",
      exact: true,
    }),
  ).toHaveValue("in_progress");
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  expect(
    await (
      await page.request.get("/api/workspace/media?path=rabbit.cube")
    ).body(),
  ).toEqual(before);
});
test("rejects an imported workflow recipe without executing or replacing the active challenge", async ({
  page,
}, info) => {
  const panel = page.getByRole("region", {
    name: "Local challenges",
    exact: true,
  });
  await page.getByLabel("Choose a file", { exact: true }).setInputFiles({
    name: `workflow-${info.project.name}.cube`,
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({
        format_version: 1,
        engine_api: 1,
        command: "touch SHOULD_NEVER_EXIST",
        answers: ["private"],
      }),
    ),
  });
  await page
    .getByRole("button", { name: "Import into workspace", exact: true })
    .click();
  await expect(
    page.getByRole("heading", {
      name: `workflow-${info.project.name}.cube`,
      exact: true,
    }),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "Import selected challenge", exact: true })
    .click();
  await expect(panel.getByRole("alert")).toContainText(
    "Workflow recipes, scripts and private answers are unsupported",
  );
  await expect(
    page.getByRole("region", { name: "Workspace files", exact: true }),
  ).toBeVisible();
});
