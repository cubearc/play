import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.skip(
  !process.env.CUBE_PLAY_DOCKER_TEST_URL,
  "This suite requires the real two-container runtime.",
);
test("executes real Cube commands through xterm and closes the process session", async ({
  page,
}) => {
  const url = process.env.CUBE_PLAY_DOCKER_TEST_URL;
  if (!url) throw new Error("Real runner session unavailable");
  await page.goto(url);
  await page
    .getByRole("button", { name: "Start terminal", exact: true })
    .click();
  await expect(
    page.getByText("Connected to Cube Runner", { exact: true }),
  ).toBeVisible();
  const input = page.getByRole("textbox", {
    name: "Cube terminal input",
    exact: true,
  });
  await input.pressSequentially("cube encode base64 hello");
  await input.press("Enter");
  await expect(
    page.getByRole("region", { name: "Cube terminal output" }),
  ).toContainText("aGVsbG8=");
  await input.pressSequentially("cube nonexistent-command");
  await input.press("Enter");
  await expect(
    page.getByRole("region", { name: "Cube terminal output" }),
  ).toContainText("unrecognized subcommand");
  const size = page.getByText(/Terminal size: \d+/);
  await expect(size).toContainText("×");
  const before = await size.textContent();
  await page.setViewportSize({
    width: test.info().project.name === "mobile" ? 430 : 900,
    height: 800,
  });
  await expect(size).not.toHaveText(before ?? "");
  const accessibility = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(accessibility.violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "End session", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Start terminal", exact: true }),
  ).toBeVisible();
});
