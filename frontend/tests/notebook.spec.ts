import AxeBuilder from "@axe-core/playwright";
import { expect, openQuickTools, openSection, test } from "./session";

test.beforeEach(async ({ page, api }) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Session unavailable");
  await page.goto(url);
  await openSection(page, "Files");
  await expect(
    page.getByRole("button", { name: "Source files", exact: true }),
  ).toBeVisible();
  const snapshot = await (await api.get("/api/investigation")).json();
  expect(
    (
      await api.put("/api/investigation/view", {
        headers: { Origin: new URL(url).origin },
        data: {
          revision: snapshot.view_revision,
          view: { store: "source", path: "", offset: 0, selected: null },
        },
      })
    ).ok(),
  ).toBe(true);
  await page.reload();
});
test("autosaves private notes and preserves exact named Lab bytes across browser reload", async ({
  page,
}, info) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const name = `Private binary ${info.project.name}`;
  await openQuickTools(page);
  const lab = page.getByRole("region", {
    name: "Text and byte Lab",
    exact: true,
  });
  await lab
    .getByRole("button", {
      name: "Discover text and byte operations",
      exact: true,
    })
    .click();
  await lab
    .getByRole("combobox", { name: "Core operation", exact: true })
    .selectOption("encoding.base64.decode");
  await lab.getByLabel("Private Lab text", { exact: true }).fill("AP+A");
  await lab
    .getByRole("button", { name: "Run Core operation", exact: true })
    .click();
  await expect(
    lab.getByLabel("Reopened result content", { exact: true }),
  ).toHaveValue("AP+A");
  await lab.getByLabel("Artifact name", { exact: true }).fill(name);
  await lab
    .getByRole("button", { name: "Save named artifact", exact: true })
    .click();
  await expect(lab.getByText(/Artifact saved locally/)).toBeVisible();
  const notes = page.getByRole("region", {
    name: "Private notes",
    exact: true,
  });
  await openSection(page, "Notes");
  await expect(
    notes.getByText("Notes saved locally", { exact: true }),
  ).toBeVisible();
  await notes.getByRole("button", { name: "New note", exact: true }).click();
  const noteName = `Hypothesis ${info.project.name}`;
  await notes.getByLabel("Note title", { exact: true }).fill(noteName);
  await notes
    .getByLabel("Note body", { exact: true })
    .fill("<script>window.noteExecuted=true</script> Private hypothesis.");
  await expect(
    notes.getByText("Notes saved locally", { exact: true }),
  ).toBeVisible();
  const library = page.getByRole("region", {
    name: "Artifact library",
    exact: true,
  });
  await openSection(page, "Artifacts");
  await library
    .getByRole("button", { name: "Refresh artifact library", exact: true })
    .click();
  await library.getByRole("button", { name, exact: true }).click();
  await expect(
    library.getByLabel("Reopened result content", { exact: true }),
  ).toHaveValue("AP+A");
  const download = page.waitForEvent("download");
  await library
    .getByRole("button", { name: "Export selected artifact", exact: true })
    .click();
  const exported = await download;
  expect(exported.suggestedFilename()).toMatch(/^cube-play-[a-f0-9-]+\.bin$/);
  const stream = await exported.createReadStream();
  if (!stream) throw new Error("Export stream missing");
  const buffers: Buffer[] = [];
  for await (const chunk of stream) buffers.push(Buffer.from(chunk));
  expect(Buffer.concat(buffers)).toEqual(Buffer.from([0, 255, 128]));
  await page.reload();
  await openSection(page, "Notes");
  await notes.getByRole("button", { name: noteName, exact: true }).click();
  await expect(notes.getByLabel("Note body", { exact: true })).toHaveValue(
    /Private hypothesis/,
  );
  expect(await page.evaluate(() => "noteExecuted" in window)).toBe(false);
  await openSection(page, "Artifacts");
  await library.getByRole("button", { name, exact: true }).click();
  await expect(
    library.getByLabel("Reopened result content", { exact: true }),
  ).toHaveValue("AP+A");
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
  expect(errors).toEqual([]);
});
test("keeps an unsaved conflicting draft until the user explicitly reloads", async ({
  page,
  api,
}, info) => {
  const id = crypto.randomUUID(),
    title = `Conflict ${info.project.name}`,
    origin = new URL(process.env.CUBE_PLAY_TEST_URL ?? "").origin;
  expect(
    (
      await api.put("/api/investigation/notes", {
        headers: { Origin: origin },
        data: { id, title, body: "Original", revision: 0 },
      })
    ).ok(),
  ).toBe(true);
  await page.reload();
  await openSection(page, "Notes");
  const notes = page.getByRole("region", {
    name: "Private notes",
    exact: true,
  });
  await notes.getByRole("button", { name: title, exact: true }).click();
  expect(
    (
      await api.put("/api/investigation/notes", {
        headers: { Origin: origin },
        data: { id, title, body: "Newer stored value", revision: 1 },
      })
    ).ok(),
  ).toBe(true);
  await notes
    .getByLabel("Note body", { exact: true })
    .fill("Unsaved local draft");
  await expect(notes.getByRole("alert")).toContainText(
    "changed in another request",
  );
  await expect(notes.getByLabel("Note body", { exact: true })).toHaveValue(
    "Unsaved local draft",
  );
  await notes
    .getByRole("button", {
      name: "Reload notes and discard local draft",
      exact: true,
    })
    .click();
  await notes.getByRole("button", { name: title, exact: true }).click();
  await expect(notes.getByLabel("Note body", { exact: true })).toHaveValue(
    "Newer stored value",
  );
});
