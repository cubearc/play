import { expect, test } from "@playwright/test";

test("builds a typed substitution exploration and restores its immutable result", async ({
  page,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await expect(
    lab.getByRole("button", { name: "New exploration" }),
  ).toBeEnabled();
  await lab.getByRole("button", { name: "New exploration" }).click();
  const title = `Substitution ${info.project.name}`;
  await lab.getByLabel("Exploration name", { exact: true }).fill(title);
  await lab
    .getByLabel("Hypothesis", { exact: true })
    .fill("A reproducible mapping with named text ports");
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
  await expect(
    lab.getByRole("button", { name: "Run selected path" }),
  ).toBeEnabled();
  await lab.getByRole("button", { name: "Run selected path" }).click();
  await expect(
    lab.getByRole("region", { name: "Selected operation outputs" }),
  ).toContainText("CAB");
  await lab
    .getByRole("combobox", { name: "Parameter form", exact: true })
    .selectOption("1");
  await lab
    .getByRole("button", { name: "Use empty text for keyword", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByRole("region", { name: "Selected operation outputs" }),
  ).toContainText("ABC");
  await lab.getByLabel("text value *", { exact: true }).fill("A");
  await lab.getByLabel("text value *", { exact: true }).clear();
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByRole("region", { name: "Selected operation outputs" }),
  ).toContainText(/"text":\s*""/);
  await page.reload();
  await expect(lab.getByLabel("Exploration name", { exact: true })).toHaveValue(
    title,
  );
  await expect(lab.getByLabel("Hypothesis", { exact: true })).toHaveValue(
    "A reproducible mapping with named text ports",
  );
  await lab.getByText(/Execution history/).click();
  await expect(lab.getByText(/succeeded ·/)).toHaveCount(3);
  await lab
    .getByRole("button", { name: "View retained output", exact: true })
    .first()
    .click();
  await expect(
    lab.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("CAB");
  await expect(
    lab.getByRole("combobox", { name: "Output port", exact: true }),
  ).toHaveValue("text");
  await expect(lab.getByText(/succeeded ·/)).toHaveCount(3);
  const response = await page.request.get("/api/explorations");
  expect(response.ok()).toBe(true);
  const value = await response.json();
  const exploration = value.explorations.find(
    (e: { title: string }) => e.title === title,
  );
  expect(exploration.steps[0].parameters).toEqual({
    source: "ABC",
    keyword: "",
    unmapped: "reject",
  });
  expect(exploration.steps[0].inputs.text).toEqual({
    type: "literal",
    value: "",
  });
  const runs = await page.request.get(
    `/api/explorations/${exploration.id}/executions`,
  );
  expect(runs.ok()).toBe(true);
  expect(await runs.json()).toHaveLength(3);
});
