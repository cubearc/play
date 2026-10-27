import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { expect, openSection, test } from "./session";

test("creates, opens and resumes workspaces through the trusted host chooser", async ({
  page,
}, info) => {
  test.skip(
    !process.env.CUBE_PLAY_CHOOSER_URL,
    "Requires the real host launcher and Docker runtime fixture",
  );
  test.setTimeout(150_000);
  const link = process.env.CUBE_PLAY_CHOOSER_URL;
  const parent = process.env.CUBE_PLAY_CHOOSER_PARENT;
  const source = process.env.CUBE_PLAY_CHOOSER_SOURCE;
  if (!link || !parent || !source)
    throw new Error("Host chooser fixture unavailable");
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const hostOrigin = new URL(link).origin;
  await page.goto(link);
  await page.getByRole("link", { name: "Workspaces", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Your workspaces", exact: true }),
  ).toBeVisible();
  await expect.poll(() => page.evaluate(() => location.hash)).toBe("");
  expect(
    await page.evaluate(() =>
      Boolean(sessionStorage.getItem("cube-play-workspace-host-capability")),
    ),
  ).toBe(true);
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.screenshot({
      path: info.outputPath("workspace-chooser.png"),
      fullPage: true,
    });

  await writeFile(join(parent, "selection"), "");
  await page.getByRole("button", { name: "Open folder", exact: true }).click();
  await expect(page.getByRole("status")).toContainText(
    "Folder selection cancelled",
  );
  await page
    .getByRole("button", { name: "New workspace", exact: true })
    .click();
  await page
    .getByLabel("Workspace folder name", { exact: true })
    .fill("../escape");
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText("portable folder name");

  const folder = `Chooser ${info.project.name} $ 空`;
  await writeFile(join(parent, "selection"), parent);
  await page.getByLabel("Workspace folder name", { exact: true }).fill(folder);
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "This workspace is empty", exact: true }),
  ).toBeVisible({ timeout: 90_000 });
  const createdOrigin = new URL(page.url()).origin;
  expect(createdOrigin).not.toBe(hostOrigin);
  expect(
    await page.evaluate(() =>
      sessionStorage.getItem("cube-play-workspace-host-capability"),
    ),
  ).toBeNull();
  expect(
    await page.evaluate(() =>
      Boolean(sessionStorage.getItem("cube-play-session-capability")),
    ),
  ).toBe(true);
  await openSection(page, "Notes");
  const notes = page.getByRole("region", {
    name: "Private notes",
    exact: true,
  });
  await notes.getByRole("button", { name: "New note", exact: true }).click();
  await notes
    .getByLabel("Note title", { exact: true })
    .fill(`Retained ${folder}`);
  await notes
    .getByLabel("Note body", { exact: true })
    .fill("Private evidence retained across workspace changes.");
  await expect(
    notes.getByText("Notes saved locally", { exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Workspaces", exact: true }).click();
  await expect(
    page
      .getByRole("button")
      .filter({ has: page.getByText(folder, { exact: true }) }),
  ).toBeEnabled();
  await writeFile(join(parent, "selection"), source);
  await page.getByRole("button", { name: "Open folder", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "investigation $private 空",
      exact: true,
    }),
  ).toBeVisible({ timeout: 90_000 });
  expect(new URL(page.url()).origin).not.toBe(createdOrigin);
  await page.getByRole("link", { name: "Workspaces", exact: true }).click();
  await page
    .getByRole("button")
    .filter({ has: page.getByText(folder, { exact: true }) })
    .click();
  await expect(
    page.getByRole("heading", { name: folder, exact: true }),
  ).toBeVisible({ timeout: 90_000 });
  expect(new URL(page.url()).origin).toBe(createdOrigin);
  await openSection(page, "Notes");
  await notes
    .getByRole("button", { name: `Retained ${folder}`, exact: true })
    .click();
  await expect(notes.getByLabel("Note body", { exact: true })).toHaveValue(
    "Private evidence retained across workspace changes.",
  );
  expect(errors).toEqual([]);
});
