import AxeBuilder from "@axe-core/playwright";
import { expect, openSection, test } from "./session";

test("keeps notes on the original execution output after a new run and browser reload", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  const name = `Evidence note ${info.project.name}`;
  await lab.getByLabel("Exploration name", { exact: true }).fill(name);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.substitution.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("ABC");
  await lab.getByLabel("source *", { exact: true }).fill("ABC");
  await lab.getByLabel("target *", { exact: true }).fill("CAB");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("CAB");
  await lab
    .getByRole("button", { name: "Annotate selected output", exact: true })
    .click();
  const notes = page.getByRole("region", {
    name: "Private notes",
    exact: true,
  });
  await notes
    .getByRole("button", {
      name: "New note for selected execution",
      exact: true,
    })
    .click();
  await notes.getByLabel("Note title", { exact: true }).fill(name);
  await notes
    .getByLabel("Note body", { exact: true })
    .fill("Keep the first result even after trying a different input.");
  await expect(
    notes.getByText("Notes saved locally", { exact: true }),
  ).toBeVisible();
  const before = await (await api.get("/api/investigation/notes")).json();
  const binding = before.find(
    (n: { title: string }) => n.title === name,
  ).target;
  expect(binding.output_port).toBe("text");
  await openSection(page, "Explorations");
  await lab.getByLabel("text value *", { exact: true }).fill("CBA");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("BAC");
  await openSection(page, "Notes");
  await notes
    .getByRole("button", { name: "Inspect linked execution", exact: true })
    .click();
  const evidence = notes.getByRole("region", {
    name: "Linked execution",
    exact: true,
  });
  await expect(
    evidence.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("CAB");
  await expect(evidence).toContainText("earlier configuration");
  await evidence
    .getByText("Recorded execution evidence", { exact: true })
    .click();
  await evidence
    .getByText("Output data and provenance", { exact: true })
    .click();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.screenshot({
      path: info.outputPath("linked-execution-note.png"),
      fullPage: true,
    });
  await page.reload();
  await notes.getByRole("button", { name, exact: true }).click();
  await notes
    .getByRole("button", { name: "Inspect linked execution", exact: true })
    .click();
  await expect(
    evidence.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("CAB");
  const after = await (await api.get("/api/investigation/notes")).json();
  expect(after.find((n: { title: string }) => n.title === name).target).toEqual(
    binding,
  );
  const run = await (
    await api.get(`/api/explorations/executions/${binding.execution_id}`)
  ).json();
  const history = await (
    await api.get(`/api/explorations/${run.exploration_id}/executions`)
  ).json();
  expect(history).toHaveLength(2);
});
