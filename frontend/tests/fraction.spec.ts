import { expect, openSection, test } from "./session";

test("runs a rail permutation and displays separate Morse padding evidence", async ({
  page,
  api,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  const catalog = await (await api.get("/api/lab/v2/operations")).json();
  expect(catalog.operations).toHaveLength(304);
  await page.goto(url);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  await lab
    .getByLabel("Exploration name", { exact: true })
    .fill(`Permutation profiles ${info.project.name}`);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.rail_fence.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab
    .getByLabel("text value *", { exact: true })
    .fill("WEAREDISCOVEREDFLEEATONCE");
  await lab.getByLabel("rails *", { exact: true }).fill("3");
  await lab.getByLabel("initial_rail *", { exact: true }).fill("0");
  await lab.getByLabel("direction *", { exact: true }).selectOption("down");
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
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("WECRLTEERDSOEEFEAOCAIVDEN");
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.fractionated_morse.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("COME AT ONCE");
  await lab
    .getByLabel("alphabet *", { exact: true })
    .fill("ROUNDTABLECFGHIJKMPQSVWXYZ");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("ciphertext");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("CBIILTMHVVFL");
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("padding_count");
  await expect(
    outputs.getByLabel("Structured output", { exact: true }),
  ).toHaveValue("2");
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("padding_ambiguous");
  await expect(
    outputs.getByLabel("Structured output", { exact: true }),
  ).toHaveValue("false");
});
