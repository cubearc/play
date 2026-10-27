import { expect, openSection, test } from "./session";

test("keeps exploration drafts across keyboard navigation and restores the saved section", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Files");
  await page.getByRole("button", { name: "Source files", exact: true }).click();
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Explore selected file", exact: true })
    .click();
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  const title = `Navigation draft ${info.project.name}`;
  await lab.getByLabel("Exploration name", { exact: true }).fill(title);
  await lab
    .getByLabel("Hypothesis", { exact: true })
    .fill("Keep the draft while inspecting other evidence");
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.substitution.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("ABC");
  await lab.getByLabel("source *", { exact: true }).fill("ABC");
  await lab.getByLabel("target *", { exact: true }).fill("CAB");
  const tab = page.getByRole("tab", { name: "Explorations", exact: true });
  await tab.focus();
  await tab.press(info.project.name === "mobile" ? "ArrowRight" : "ArrowDown");
  await expect(
    page.getByRole("tabpanel", { name: "Notes", exact: true }),
  ).toBeVisible();
  await expect(lab).not.toBeVisible();
  await openSection(page, "Explorations");
  await expect(lab.getByLabel("Exploration name", { exact: true })).toHaveValue(
    title,
  );
  await expect(lab.getByLabel("text value *", { exact: true })).toHaveValue(
    "ABC",
  );
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("CAB");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.evaluate(() => scrollTo(0, 0));
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.screenshot({
      path: info.outputPath("workspace-exploration.png"),
      fullPage: true,
    });
  await page.reload();
  await expect(
    page.getByRole("tab", { name: "Explorations", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(lab.getByLabel("Exploration name", { exact: true })).toHaveValue(
    title,
  );
  const snapshot = await (await api.get("/api/investigation")).json();
  expect(snapshot.view.panel).toBe("explorations");
  expect(snapshot.view.selected.path).toBe("sample.png");
  expect(
    await page
      .getByRole("button", { name: "From challenge", exact: true })
      .count(),
  ).toBe(0);
  await openSection(page, "Files");
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.evaluate(() => scrollTo(0, 0));
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.screenshot({
      path: info.outputPath("workspace-files.png"),
      fullPage: true,
    });
});
