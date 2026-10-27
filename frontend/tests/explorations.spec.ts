import { expect, openSection, test } from "./session";

test("chains real image decoding and channel outputs through typed ports", async ({
  page,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Files");
  await page.getByRole("button", { name: "Source files", exact: true }).click();
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  await lab
    .getByLabel("Exploration name", { exact: true })
    .fill(`Image channel ${info.project.name}`);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("image.decode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab
    .getByRole("combobox", { name: "input · bytes", exact: true })
    .selectOption("file");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByRole("region", { name: "Exact raster viewer", exact: true }),
  ).toContainText("2 × 2");
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("image.channel.extract");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab
    .getByRole("combobox", { name: "input · image.samples", exact: true })
    .selectOption({ label: "image.decode → output" });
  await lab.getByLabel("channel *", { exact: true }).selectOption("red");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  const outputs = lab.getByRole("region", {
    name: "Selected operation outputs",
    exact: true,
  });
  await expect(
    outputs.getByRole("combobox", { name: "Output port", exact: true }),
  ).toHaveValue("output");
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("image");
  await expect(outputs.getByRole("table")).toContainText("20");
  await expect(
    outputs.getByRole("columnheader", { name: "gray", exact: true }),
  ).toBeVisible();
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("data");
  await expect(
    outputs.getByLabel("Base64 byte output", { exact: true }),
  ).toHaveValue(Buffer.from([20, 230, 120, 40]).toString("base64"));
  await expect(
    outputs.getByRole("region", { name: "Replay availability", exact: true }),
  ).toContainText("Recorded inputs are currently available.");
});

test("builds a typed substitution exploration and restores its immutable result", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Explorations");
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
  await expect(
    lab.getByRole("region", { name: "Replay availability", exact: true }),
  ).toContainText("Recorded inputs are currently available.");
  await expect(lab.getByText(/succeeded ·/)).toHaveCount(3);
  const response = await api.get("/api/explorations");
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
  const runs = await api.get(`/api/explorations/${exploration.id}/executions`);
  expect(runs.ok()).toBe(true);
  expect(await runs.json()).toHaveLength(3);
});
