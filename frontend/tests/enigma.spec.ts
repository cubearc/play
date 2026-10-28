import { expect, openSection, test } from "./session";

test("runs Enigma with private settings and displays text, final windows and trace", async ({
  page,
}) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await openSection(page, "Explorations");
  const lab = page.getByRole("region", { name: "Explorations", exact: true });
  await lab
    .getByRole("button", { name: "New exploration", exact: true })
    .click();
  await lab
    .getByLabel("Exploration name", { exact: true })
    .fill("Enigma historical vector");
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.enigma");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("AAAAA");
  await lab
    .getByLabel("profile *", { exact: true })
    .selectOption({ label: "enigma_i" });
  await lab
    .getByLabel("reflector *", { exact: true })
    .selectOption({ label: "B" });
  await lab
    .getByLabel("rotors * · private for this run", { exact: true })
    .fill('["I","II","III"]');
  await lab
    .getByLabel("rings * · private for this run", { exact: true })
    .fill("[1,1,1]");
  await lab
    .getByLabel("windows * · private for this run", { exact: true })
    .fill("AAA");
  await lab
    .getByLabel("plugboard * · private for this run", { exact: true })
    .fill("[]");
  await lab.getByLabel("trace", { exact: true }).check();
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
  const port = outputs.getByRole("combobox", {
    name: "Output port",
    exact: true,
  });
  await port.selectOption("text");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("BDZGO");
  await port.selectOption("final_windows");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("AAF");
  await port.selectOption("trace");
  await expect(outputs).toContainText("AAB");
  await expect(outputs).toContainText("AAF");
  await expect(
    lab.getByText(
      "Execution recorded locally. Private output was not retained.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    lab.getByLabel("windows * · private for this run", { exact: true }),
  ).toHaveValue("");
});
