import { expect, test } from "@playwright/test";

test("runs canonical historical ciphers and displays separate filler evidence", async ({
  page,
}, info) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  await lab
    .getByLabel("Exploration name", { exact: true })
    .fill(`Historical profiles ${info.project.name}`);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.gronsfeld.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("THISONE");
  await lab.getByLabel("key *", { exact: true }).fill("9321492");
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
  ).toHaveValue("CKKTSWG");
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.playfair.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("XX");
  await lab
    .getByLabel("square *", { exact: true })
    .fill("ABCDEFGHIKLMNOPQRSTUVWXYZ");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("prepared");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("XQXQ");
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("fillers");
  await expect(outputs).toContainText(/1,\s*3/);
});
