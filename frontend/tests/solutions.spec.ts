import AxeBuilder from "@axe-core/playwright";
import { expect, openSection, test } from "./session";

test("preserves a private derivation while exploration continues and reopens immutable revisions", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  const origin = new URL(url).origin,
    external: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).origin !== origin) external.push(request.url());
  });
  await page.goto(url);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  const title = `Private derivation ${info.project.name}`;
  await lab.getByLabel("Exploration name", { exact: true }).fill(title);
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
    lab.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("CAB");
  await lab
    .getByRole("button", { name: "Preserve as solution", exact: true })
    .click();
  await expect(
    page.getByRole("tab", { name: "Solutions", exact: true }),
  ).toBeFocused();
  const panel = page.getByRole("region", {
    name: "Private solutions",
    exact: true,
  });
  await panel
    .getByRole("button", {
      name: "Create finding from selected output",
      exact: true,
    })
    .click();
  await panel.getByLabel("Solution title", { exact: true }).fill(title);
  await panel
    .getByRole("textbox", { name: "Solution explanation", exact: true })
    .fill(
      "The explicit alphabet maps ABC to CAB. Preserve this result while testing another input.",
    );
  await panel.getByLabel("Mark as resolved locally", { exact: true }).check();
  await panel
    .getByRole("button", { name: "Open Explorations", exact: true })
    .click();
  await openSection(page, "Solutions");
  await expect(
    panel.getByRole("textbox", { name: "Solution explanation", exact: true }),
  ).toHaveValue(/maps ABC to CAB/);
  await panel
    .getByRole("button", { name: "Save private solution", exact: true })
    .click();
  const saved = panel.getByRole("region", {
    name: "Saved solution",
    exact: true,
  });
  await expect(
    saved.getByText("Resolved locally · Private · Unverified", { exact: true }),
  ).toBeVisible();
  await saved
    .getByRole("button", { name: "Inspect final value", exact: true })
    .click();
  await expect(
    saved.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("CAB");
  const entries = await (await api.get("/api/solutions")).json();
  const id = entries.solutions.find(
    (s: { title: string }) => s.title === title,
  ).id;
  const original = await (await api.get(`/api/solutions/${id}`)).json();
  expect(original.solution.configurations).toHaveLength(1);
  expect(original.solution.executions).toHaveLength(1);
  const graph = original.solution.configurations[0].id;
  await saved
    .getByRole("button", { name: "Continue in Explorations", exact: true })
    .click();
  await lab.getByLabel("text value *", { exact: true }).fill("CBA");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("BAC");
  await openSection(page, "Solutions");
  await saved
    .getByRole("button", { name: "Check solution evidence again", exact: true })
    .click();
  await saved
    .getByRole("button", { name: "Inspect final value", exact: true })
    .click();
  await expect(
    saved.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("CAB");
  await saved
    .getByRole("button", { name: "Revise this solution", exact: true })
    .click();
  await panel
    .getByRole("textbox", { name: "Solution explanation", exact: true })
    .fill(
      "A second input produced BAC; this solution still describes the earlier CAB result.",
    );
  await panel
    .getByRole("button", { name: "Save private solution", exact: true })
    .click();
  await expect(
    saved.getByRole("combobox", { name: "Solution revision", exact: true }),
  ).toHaveValue("2");
  await saved
    .getByRole("combobox", { name: "Solution revision", exact: true })
    .selectOption("1");
  await expect(
    saved.getByRole("textbox", { name: "Saved explanation", exact: true }),
  ).toHaveValue(original.solution.explanation);
  await saved
    .getByText(/Selected execution path · 1 retained results/, { exact: true })
    .click();
  await saved.getByText("Final reference and digest", { exact: true }).click();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  if (process.env.CUBE_PLAY_CAPTURE_UI === "1")
    await page.screenshot({
      path: info.outputPath("private-solution.png"),
      fullPage: true,
    });
  await page.reload();
  await panel
    .getByRole("button", {
      name: `${title} · revision 2 · Resolved locally`,
      exact: true,
    })
    .click();
  await expect(
    saved.getByRole("textbox", { name: "Saved explanation", exact: true }),
  ).toHaveValue(/second input produced BAC/);
  await saved
    .getByRole("button", { name: "Inspect final value", exact: true })
    .click();
  await expect(
    saved.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("CAB");
  const after = await (await api.get(`/api/solutions/${id}?revision=1`)).json();
  expect(after.solution).toEqual(original.solution);
  expect(
    await (await api.get(`/api/explorations/${graph}/executions`)).json(),
  ).toHaveLength(2);
  expect(external).toEqual([]);
});

test("associates an existing local byte candidate without submitting or changing progress", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  const headers = { Origin: new URL(url).origin };
  const challengeResponse = await api.post("/api/challenges/import", {
    headers,
    data: { store: "source", path: "rabbit.cube" },
  });
  expect(challengeResponse.ok()).toBe(true);
  const challenge = await challengeResponse.json();
  const key = challenge.active.definition.key,
    stage = "inspect";
  const id = crypto.randomUUID();
  const result = await api.post("/api/lab/v2/execute", {
    headers,
    data: {
      schema_version: 2,
      id,
      operation: "encoding.base64.decode",
      operation_version: 1,
      inputs: { input: { type: "literal", value: "AP8=" } },
      parameters: {},
    },
  });
  expect(result.ok()).toBe(true);
  expect(
    (
      await api.put("/api/lab/library", {
        headers,
        data: { id, revision: 0, title: `Solution bytes ${info.project.name}` },
      })
    ).ok(),
  ).toBe(true);
  const prior = await (
    await api.get(`/api/challenges/candidates?key=${key}&stage=${stage}`)
  ).json();
  const associated = await api.put("/api/challenges/candidates", {
    headers,
    data: {
      key,
      stage,
      result_id: id,
      revision: prior.candidate?.revision ?? 0,
      selection: {
        kind: "field",
        pointer: "/output/base64",
        conversion: "base64_bytes",
      },
    },
  });
  expect(associated.ok()).toBe(true);
  const candidate = await associated.json();
  const before = await (await api.get("/api/challenges")).json();
  await page.goto(url);
  await openSection(page, "Solutions");
  const panel = page.getByRole("region", {
    name: "Private solutions",
    exact: true,
  });
  await panel
    .getByRole("button", {
      name: "New finding from retained results",
      exact: true,
    })
    .click();
  await panel
    .getByRole("button", {
      name: new RegExp(`encoding.base64.decode ·.*${id.slice(0, 8)}`),
    })
    .click();
  await panel
    .getByLabel("Solution title", { exact: true })
    .fill(`Local byte solution ${info.project.name}`);
  await panel
    .getByRole("textbox", { name: "Solution explanation", exact: true })
    .fill("The existing candidate selects exactly the bytes 00 ff.");
  await panel
    .getByRole("button", { name: "Choose local stage candidate", exact: true })
    .click();
  await panel
    .getByRole("combobox", { name: "Candidate stage", exact: true })
    .selectOption(stage);
  await expect(
    panel.getByRole("combobox", { name: "Candidate revision", exact: true }),
  ).toHaveValue(String(candidate.candidate.revision));
  await panel
    .getByRole("button", { name: "Use candidate revision", exact: true })
    .click();
  await expect(
    panel.getByLabel("Final field JSON pointer", { exact: true }),
  ).toHaveValue("/base64");
  await panel
    .getByRole("button", { name: "Save private solution", exact: true })
    .click();
  const saved = panel.getByRole("region", {
    name: "Saved solution",
    exact: true,
  });
  await saved
    .getByRole("button", { name: "Inspect final value", exact: true })
    .click();
  await expect(
    saved.getByRole("textbox", { name: "Base64 byte output", exact: true }),
  ).toHaveValue("AP8=");
  await saved
    .getByText("Frozen local stage candidate", { exact: true })
    .click();
  await expect(
    saved.getByRole("textbox", { name: "Frozen stage candidate", exact: true }),
  ).toHaveValue(/private_unverified/);
  expect(await (await api.get("/api/challenges")).json()).toEqual(before);
  expect(
    await (
      await api.get(`/api/challenges/candidates?key=${key}&stage=${stage}`)
    ).json(),
  ).toEqual(candidate);
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});
