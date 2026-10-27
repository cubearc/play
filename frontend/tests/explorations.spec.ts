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

for (const mode of ["Flow", "Steps"]) {
  test(`reuses a retained structured field through ${mode} with exact replay evidence`, async ({
    page,
    api,
  }, info) => {
    const origin = new URL(process.env.CUBE_PLAY_TEST_URL ?? "").origin;
    const seed = await api.post("/api/lab/v2/execute", {
      headers: { Origin: origin },
      data: {
        schema_version: 2,
        id: crypto.randomUUID(),
        operation: "text.josephus.extract",
        operation_version: 1,
        inputs: { input: { type: "literal", value: "ABCDE" } },
        parameters: { steps: [1], count: 2 },
      },
    });
    expect(seed.ok()).toBeTruthy();
    const result = await seed.json();
    const id = result.result.id;
    const title = `Extracted letters ${mode} ${info.project.name}`;
    expect(
      (
        await api.put("/api/lab/library", {
          headers: { Origin: origin },
          data: { id, title, revision: 0 },
        })
      ).ok(),
    ).toBeTruthy();
    await page.goto(process.env.CUBE_PLAY_TEST_URL ?? "");
    await openSection(page, "Explorations");
    const lab = page.getByRole("region", { name: "Explorations", exact: true });
    await lab
      .getByRole("button", { name: "New exploration", exact: true })
      .click();
    await lab
      .getByLabel("Exploration name", { exact: true })
      .fill(`Field chain ${mode} ${info.project.name}`);
    await lab
      .getByRole("combobox", { name: "Operation", exact: true })
      .selectOption("text.reverse");
    await lab.getByRole("button", { name: "Add step", exact: true }).click();
    await lab.getByRole("button", { name: mode, exact: true }).click();
    await lab
      .getByRole("button", { name: "Load saved inputs", exact: true })
      .click();
    await lab
      .getByRole("combobox", { name: "input · text", exact: true })
      .selectOption(`artifact:${id}:output`);
    await lab
      .getByRole("textbox", { name: "Field JSON Pointer", exact: true })
      .fill("/extracted");
    await lab
      .getByRole("button", { name: "Save configuration", exact: true })
      .click();
    await lab
      .getByRole("button", { name: "Run selected path", exact: true })
      .click();
    await expect(
      lab.getByLabel("Exact text output", { exact: true }),
    ).toHaveValue("BA");
    await page.reload();
    await expect(
      lab.getByLabel("Field JSON Pointer", { exact: true }),
    ).toHaveValue("/extracted");
    await lab.getByText(/Execution history ·/).click();
    await lab
      .getByRole("button", { name: "View retained output", exact: true })
      .click();
    await expect(
      lab.getByLabel("Exact text output", { exact: true }),
    ).toHaveValue("BA");
  });
}

test("associates a saved exploration with a local stage without executing or changing progress", async ({
  page,
  api,
}, info) => {
  const link = process.env.CUBE_PLAY_TEST_URL;
  if (!link) throw new Error("Local session unavailable");
  const origin = new URL(link).origin;
  const response = await api.post("/api/challenges/import", {
    headers: { Origin: origin },
    data: { store: "source", path: "rabbit.cube" },
  });
  expect(response.ok()).toBeTruthy();
  const before = await response.json();
  const key = before.active.definition.key;
  const stage = before.active.definition.manifest.stages[0].id;
  await page.goto(link);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  const title = `Local stage exploration ${info.project.name}`;
  await lab.getByLabel("Exploration name", { exact: true }).fill(title);
  await lab
    .getByRole("button", { name: "Load local stages", exact: true })
    .click();
  await expect(
    lab.getByRole("option", {
      name: `${before.active.definition.manifest.title} → ${before.active.definition.manifest.stages[0].title}`,
      exact: true,
    }),
  ).toHaveAttribute("value", `${key}:${stage}`);
  await lab
    .getByRole("combobox", { name: "Exploration scope", exact: true })
    .selectOption(`${key}:${stage}`);
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await expect(
    lab.getByRole("button", { name: "Save configuration", exact: true }),
  ).toBeDisabled();
  const graphs = await (await api.get("/api/explorations")).json();
  const graph = graphs.explorations.find(
    (item: { title: string }) => item.title === title,
  );
  expect(graph.scope).toEqual({ challenge: key, stage });
  expect(
    await (await api.get(`/api/explorations/${graph.id}/executions`)).json(),
  ).toEqual([]);
  const after = await (await api.get("/api/challenges")).json();
  expect(after.active.progress).toEqual(before.active.progress);
  await page.reload();
  await expect(
    lab.getByRole("combobox", { name: "Exploration scope", exact: true }),
  ).toHaveValue(`${key}:${stage}`);
});
