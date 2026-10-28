import { expect, openSection, test } from "./session";

test("runs raw AES with transient private bytes and a redacted key", async ({
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
    .fill("FIPS raw block vector");
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.aes.encrypt_blocks");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab
    .getByLabel("input value *", { exact: true })
    .fill('{"base64":"ABEiM0RVZneImaq7zN3u/w=="}');
  await lab
    .getByLabel("key_hex * · private for this run", { exact: true })
    .fill("000102030405060708090a0b0c0d0e0f");
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
    outputs.getByLabel("Base64 byte output", { exact: true }),
  ).toHaveValue("acTg2Gp7BDDYzbeAcLTFWg==");
  await expect(
    lab.getByText(
      "Execution recorded locally. Private output was not retained.",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    lab.getByLabel("key_hex * · private for this run", { exact: true }),
  ).toHaveValue("");
});
