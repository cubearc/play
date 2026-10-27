import AxeBuilder from "@axe-core/playwright";
import { expect, openSection, test } from "./session";

test("retains a private Core output only for the explicitly chosen run", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  const name = `Private plaintext ${info.project.name}`;
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  await lab.getByLabel("Exploration name", { exact: true }).fill(name);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.solitaire.decode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("EXKYIZSGEH");
  const deck = lab.getByRole("textbox", {
    name: "deck * · private for this run",
    exact: true,
  });
  const cards = JSON.stringify(Array.from({ length: 54 }, (_, i) => i + 1));
  await deck.fill(cards);
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  const keep = lab.getByRole("checkbox", {
    name: "Keep this private output for this run",
    exact: true,
  });
  await expect(keep).not.toBeChecked();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("AAAAAAAAAA");
  await expect(
    lab.getByText(
      "Execution recorded locally. Private output was not retained.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    lab.getByRole("button", { name: "Preserve as solution", exact: true }),
  ).toHaveCount(0);
  await expect(deck).toHaveValue("");
  const configurations = await (await api.get("/api/explorations")).json();
  const graph = configurations.explorations.find(
    (e: { title: string }) => e.title === name,
  );
  expect(graph.steps[0].parameters.deck).toBeUndefined();
  expect(graph.steps[0].retain_private_output).toBeUndefined();
  const first = await (
    await api.get(`/api/explorations/${graph.id}/executions`)
  ).json();
  expect(first).toHaveLength(1);
  expect(first[0].evidence.retained).toBe(false);
  expect(first[0].evidence.result_id).toBeNull();
  await deck.fill(cards);
  await keep.check();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByText("Execution and output preserved locally", { exact: true }),
  ).toBeVisible();
  await expect(keep).not.toBeChecked();
  await expect(deck).toHaveValue("");
  const history = await (
    await api.get(`/api/explorations/${graph.id}/executions`)
  ).json();
  expect(history).toHaveLength(2);
  expect(history[1].evidence.retained).toBe(true);
  const artifact = await (
    await api.get(`/api/lab/results/${history[1].evidence.result_id}`)
  ).json();
  expect(artifact.result.provenance.parameters).toContainEqual({
    name: "deck",
    redacted: true,
    replay: "required",
  });
  expect(JSON.stringify(artifact.result)).not.toContain(cards);
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await expect(
    lab.getByRole("button", { name: "Preserve as solution", exact: true }),
  ).toBeEnabled();
  await expect(
    lab.getByRole("textbox", { name: "Exact text output", exact: true }),
  ).toHaveValue("AAAAAAAAAA");
  expect((await api.get(`/api/lab/results/${first[0].id}`)).ok()).toBe(false);
  expect(
    await (await api.get(`/api/explorations/${graph.id}/executions`)).json(),
  ).toEqual(history);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
