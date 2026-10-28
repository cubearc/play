import { expect, openSection, test } from "./session";

const profiles = [
  {
    operation: "cipher.interrupted_key.encode",
    input: "AAAAA",
    key: "BC",
    field: "resets",
    value: "[3]",
    expected: "BCBBC",
  },
  {
    operation: "cipher.progressive_key.encode",
    input: "AAAAA",
    key: "BC",
    field: "increment",
    value: "1",
    expected: "BCCDD",
  },
  {
    operation: "cipher.vigenere_multiplicative.encode",
    input: "IAMSTUDYING",
    key: "MOBILE",
    field: "multiplier",
    value: "7",
    expected: "KUNACMBGLRP",
  },
];

for (const profile of profiles) {
  test(`configures and executes ${profile.operation} through the typed UI`, async ({
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
      .fill(profile.operation);
    await lab
      .getByRole("combobox", { name: "Operation", exact: true })
      .selectOption(profile.operation);
    await lab.getByRole("button", { name: "Add step", exact: true }).click();
    await lab.getByLabel("text value *", { exact: true }).fill(profile.input);
    await lab.getByLabel("key *", { exact: true }).fill(profile.key);
    const field = lab.getByLabel(`${profile.field} *`, { exact: true });
    if (profile.field === "multiplier")
      await field.selectOption({ label: profile.value });
    else await field.fill(profile.value);
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
    ).toHaveValue(profile.expected);
  });
}
