import AxeBuilder from "@axe-core/playwright";
import { expect, openQuickTools, openSection, test } from "./session";

test("associates exact Lab bytes and explicit structured fields, then restores candidates", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Session unavailable");
  const origin = new URL(url).origin;
  const external: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).origin !== origin) external.push(request.url());
  });
  await page.goto(url);
  await openSection(page, "Files");
  await page.getByRole("button", { name: "Source files", exact: true }).click();
  await page
    .getByRole("button", { name: "Inspect rabbit.cube", exact: true })
    .click();
  const challenges = page.getByRole("region", {
    name: "Local challenges",
    exact: true,
  });
  await openSection(page, "Local challenges");
  await challenges
    .getByRole("button", { name: "Import selected challenge", exact: true })
    .click();
  await challenges
    .getByRole("button", { name: "Inspect evidence", exact: true })
    .click();
  await openSection(page, "Files");
  await page.getByRole("button", { name: "Source files", exact: true }).click();
  const textLab = page.getByRole("region", {
    name: "Text and byte Lab",
    exact: true,
  });
  await openQuickTools(page);
  await textLab
    .getByRole("button", {
      name: "Discover text and byte operations",
      exact: true,
    })
    .click();
  await textLab
    .getByRole("combobox", { name: "Core operation", exact: true })
    .selectOption("encoding.base64.decode");
  await textLab.getByLabel("Private Lab text", { exact: true }).fill("AP+A");
  await textLab
    .getByRole("button", { name: "Run Core operation", exact: true })
    .click();
  await expect(
    textLab.getByLabel("Reopened result content", { exact: true }),
  ).toHaveValue("AP+A");
  const bytesName = `Candidate bytes ${info.project.name}`;
  await textLab.getByLabel("Artifact name", { exact: true }).fill(bytesName);
  await textLab
    .getByRole("button", { name: "Save named artifact", exact: true })
    .click();
  await expect(textLab.getByText(/Artifact saved locally/)).toBeVisible();
  const association = textLab.getByRole("region", {
    name: "Stage candidate association",
    exact: true,
  });
  await association
    .getByRole("button", { name: "Use result as stage candidate", exact: true })
    .click();
  await association
    .getByRole("button", { name: "Associate private candidate", exact: true })
    .click();
  await expect(
    association.getByText(
      /Private candidate revision \d+ · bytes · 3 bytes · unverified/,
    ),
  ).toBeVisible();
  await openSection(page, "Files");
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  const source = await (
    await api.get("/api/workspace/media?path=sample.png")
  ).body();
  await page
    .getByRole("button", { name: "Open visual Lab", exact: true })
    .click();
  const visual = page.getByRole("region", { name: "Visual Lab", exact: true });
  await visual
    .getByRole("button", { name: "Decode image in Core", exact: true })
    .click();
  await visual
    .getByRole("combobox", { name: "Analysis channel", exact: true })
    .selectOption("green");
  await visual
    .getByRole("button", { name: "Extract channel", exact: true })
    .click();
  await expect(
    visual.getByLabel("Structured result content", { exact: true }),
  ).toHaveValue(/preview_png_base64/);
  await visual
    .getByLabel("Artifact name", { exact: true })
    .fill(`Candidate channel ${info.project.name}`);
  await visual
    .getByRole("button", { name: "Save named artifact", exact: true })
    .click();
  await expect(visual.getByText(/Artifact saved locally/)).toBeVisible();
  const field = visual.getByRole("region", {
    name: "Stage candidate association",
    exact: true,
  });
  await field
    .getByRole("button", { name: "Use result as stage candidate", exact: true })
    .click();
  await field
    .getByRole("combobox", { name: "Candidate selection", exact: true })
    .selectOption("field");
  await field
    .getByLabel("JSON field pointer", { exact: true })
    .fill("/preview_png_base64");
  await field
    .getByRole("combobox", { name: "Field interpretation", exact: true })
    .selectOption("base64_bytes");
  await field
    .getByRole("button", { name: "Associate private candidate", exact: true })
    .click();
  await expect(
    field.getByText(/Private candidate revision \d+ · bytes/),
  ).toBeVisible();
  await field
    .getByRole("combobox", { name: "Candidate stage", exact: true })
    .selectOption("record");
  await field
    .getByLabel("JSON field pointer", { exact: true })
    .fill("/channels/0");
  await field
    .getByRole("combobox", { name: "Field interpretation", exact: true })
    .selectOption("text");
  await field
    .getByRole("button", { name: "Associate private candidate", exact: true })
    .click();
  await expect(
    field.getByText(
      /Private candidate revision \d+ · text · 5 bytes · unverified/,
    ),
  ).toBeVisible();
  await page.reload();
  await openSection(page, "Local challenges");
  await challenges
    .getByRole("button", { name: "Record a candidate", exact: true })
    .click();
  const restored = challenges.getByRole("region", {
    name: "Stage candidate",
    exact: true,
  });
  await expect(
    restored.getByText(
      /Private candidate revision \d+ · text · 5 bytes · unverified/,
    ),
  ).toBeVisible();
  await expect(
    restored.getByLabel("Candidate provenance history", { exact: true }),
  ).toHaveValue(/\/channels\/0/);
  const library = page.getByRole("region", {
    name: "Artifact library",
    exact: true,
  });
  await openSection(page, "Artifacts");
  await library.getByRole("button", { name: bytesName, exact: true }).click();
  await expect(
    library.getByLabel("Reopened result content", { exact: true }),
  ).toHaveValue("AP+A");
  expect(
    await (await api.get("/api/workspace/media?path=sample.png")).body(),
  ).toEqual(source);
  expect(external).toEqual([]);
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1") {
    await openSection(page, "Local challenges");
    await restored.scrollIntoViewIfNeeded();
    await page.screenshot({
      path: info.outputPath("private-stage-candidate.png"),
    });
  }
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
});
