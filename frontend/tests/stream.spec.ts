import { expect, openSection, test } from "./session";

test("runs a private stream vector and clears both byte inputs without retaining output", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  const title = `Raw stream ${info.project.name}`;
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  await lab.getByLabel("Exploration name", { exact: true }).fill(title);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.chacha20");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  const input = lab.getByLabel("input value *", { exact: true });
  const key = lab.getByLabel("key value *", { exact: true });
  await input.fill('{"base64":"AA=="}');
  await key.fill('{"base64":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}');
  await lab
    .getByLabel("nonce *", { exact: true })
    .fill("[0,0,0,0,0,0,0,0,0,0,0,0]");
  await lab.getByLabel("counter *", { exact: true }).fill("0");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await expect(
    lab.getByLabel("Keep this private output for this run", { exact: true }),
  ).not.toBeChecked();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await expect(
    lab.getByLabel("Base64 byte output", { exact: true }),
  ).toHaveValue("dg==");
  await expect(
    lab.getByText(
      "Execution recorded locally. Private output was not retained.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(input).toHaveValue("");
  await expect(key).toHaveValue("");
  const configurations = await (await api.get("/api/explorations")).json();
  const graph = configurations.explorations.find(
    (entry: { title: string }) => entry.title === title,
  );
  expect(graph.steps[0].inputs).toEqual({});
  const history = await (
    await api.get(`/api/explorations/${graph.id}/executions`)
  ).json();
  expect(history[0].evidence.retained).toBe(false);
  expect(history[0].evidence.inputs).toEqual([]);
});
