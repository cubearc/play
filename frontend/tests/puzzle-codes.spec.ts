import { expect, openSection, test } from "./session";

test("preserves caller book sources and symbolic alias evidence in named output viewers", async ({
  page,
}, info) => {
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
    .fill(`Caller profiles ${info.project.name}`);
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("cipher.book.encode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab
    .getByLabel("source value *", { exact: true })
    .fill("aba cab\nA😀 a");
  await lab.getByLabel("text value *", { exact: true }).fill("aaa😀");
  await lab
    .getByLabel("scheme *", { exact: true })
    .selectOption("word_character");
  await lab.getByLabel("index_base *", { exact: true }).fill("1");
  await lab
    .getByLabel("matching *", { exact: true })
    .selectOption("round_robin");
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
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("coordinates");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("1:1,1:3,2:2,3:2");
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("source_sha256");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue(
    "b66cd26f0139b0db53ffb85a65a095972bdd06fee0bc0c6dd6ceb271c313451f",
  );
  await lab
    .getByRole("combobox", { name: "Operation", exact: true })
    .selectOption("symbol.tokens.decode");
  await lab.getByRole("button", { name: "Add step", exact: true }).click();
  await lab.getByLabel("text value *", { exact: true }).fill("triangle|○|△");
  await lab.getByLabel("table *", { exact: true }).fill(
    JSON.stringify({
      source_id: "synthetic:original-v1",
      profile_id: "example-token-table",
      license_id: "original-fixture",
      entries: [
        { token: "△", value: "A", aliases: ["triangle"] },
        { token: "○", value: "B", aliases: [] },
      ],
    }),
  );
  await lab
    .getByLabel("boundaries *", { exact: true })
    .fill('{"kind":"separated","separator":"|"}');
  await lab.getByLabel("output_separator *", { exact: true }).fill("|");
  await lab
    .getByRole("button", { name: "Save configuration", exact: true })
    .click();
  await lab
    .getByRole("button", { name: "Run selected path", exact: true })
    .click();
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("text");
  await expect(
    outputs.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("A|B|A");
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("records");
  await expect
    .poll(async () =>
      JSON.parse(
        await outputs
          .getByLabel("Structured output", { exact: true })
          .inputValue(),
      ),
    )
    .toEqual([
      {
        input_token: "triangle",
        canonical_token: "△",
        output_token: "A",
        byte_offset: 0,
        scalar_offset: 0,
        alias_used: true,
      },
      {
        input_token: "○",
        canonical_token: "○",
        output_token: "B",
        byte_offset: 9,
        scalar_offset: 9,
        alias_used: false,
      },
      {
        input_token: "△",
        canonical_token: "△",
        output_token: "A",
        byte_offset: 13,
        scalar_offset: 11,
        alias_used: false,
      },
    ]);
  await outputs
    .getByRole("combobox", { name: "Output port", exact: true })
    .selectOption("bijective");
  await expect(
    outputs.getByLabel("Structured output", { exact: true }),
  ).toHaveValue("false");
});
