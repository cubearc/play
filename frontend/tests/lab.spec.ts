import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "./session";

let previousView: unknown;
test.beforeEach(async ({ page, api }) => {
  const url = process.env.CUBE_PLAY_TEST_URL;
  if (!url) throw new Error("Local session unavailable");
  await page.goto(url);
  await expect(
    page.getByRole("button", { name: "Source files", exact: true }),
  ).toBeVisible();
  previousView = (await (await api.get("/api/investigation")).json()).view;
  await page.getByRole("button", { name: "Source files", exact: true }).click();
});
test.afterEach(async ({ api }) => {
  const snapshot = await (await api.get("/api/investigation")).json();
  const origin = new URL(process.env.CUBE_PLAY_TEST_URL ?? "").origin;
  const response = await api.put("/api/investigation/view", {
    headers: { Origin: origin },
    data: { revision: snapshot.view_revision, view: previousView },
  });
  expect(response.ok()).toBe(true);
});

test("inspects exact Core samples, histogram, channels and transformed results", async ({
  page,
  api,
}) => {
  const external: string[] = [],
    errors: string[] = [];
  const origin = new URL(process.env.CUBE_PLAY_TEST_URL ?? "").origin;
  page.on("request", (request) => {
    if (new URL(request.url()).origin !== origin) external.push(request.url());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  const bytes = await (
    await api.get("/api/workspace/media?path=sample.png")
  ).body();
  await page
    .getByRole("button", { name: "Inspect sample.png", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Open visual Lab", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Decode image in Core", exact: true })
    .click();
  const lab = page.getByRole("region", { name: "Visual Lab", exact: true });
  await expect(lab.getByText("2 × 2 · rgb · 8 bits")).toBeVisible();
  const original = lab.getByRole("img", {
    name: "Original Core-decoded raster",
    exact: true,
  });
  await expect
    .poll(() =>
      original.evaluate(
        (node) => node instanceof HTMLImageElement && node.naturalWidth,
      ),
    )
    .toBe(2);
  await lab.getByRole("button", { name: "Read pixel", exact: true }).click();
  await expect(
    lab.getByText("Core pixel (0, 0) · red, green, blue: 20, 80, 50", {
      exact: true,
    }),
  ).toBeVisible();
  await lab.getByLabel("Pixel X", { exact: true }).focus();
  await page.keyboard.press("ArrowUp");
  await lab.getByRole("button", { name: "Read pixel", exact: true }).click();
  await expect(
    lab.getByText("Core pixel (1, 0) · red, green, blue: 230, 220, 190", {
      exact: true,
    }),
  ).toBeVisible();
  await lab
    .getByRole("combobox", { name: "Pixel zoom", exact: true })
    .selectOption("32");
  await lab.getByLabel("Pixel grid (8× or higher)", { exact: true }).uncheck();
  await lab
    .getByRole("button", { name: "Compute histogram", exact: true })
    .click();
  await lab.getByText("Histogram values", { exact: true }).click();
  const table = lab.getByRole("table", {
    name: "Core sample counts",
    exact: true,
  });
  await expect(table.getByRole("row")).toHaveCount(5);
  for (const value of [20, 40, 120, 230])
    await expect(
      table.getByRole("rowheader", { name: String(value), exact: true }),
    ).toBeVisible();
  await expect(
    lab.getByLabel("Structured result content", { exact: true }),
  ).toHaveValue(/"pixels": 4/);
  await lab
    .getByRole("combobox", { name: "Analysis channel", exact: true })
    .selectOption("green");
  await lab
    .getByRole("button", { name: "Extract channel", exact: true })
    .click();
  await expect(
    lab.getByRole("img", {
      name: "Core-generated grayscale channel preview",
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    lab.getByLabel("Structured result content", { exact: true }),
  ).toHaveValue(/"green"/);
  const channelData = JSON.parse(
    await lab
      .getByLabel("Structured result content", { exact: true })
      .inputValue(),
  );
  const catalog = await (await api.get("/api/lab/operations")).json();
  async function coreProbe(
    operation: string,
    input: unknown,
    parameters: unknown[],
    retained = true,
  ) {
    const descriptor = catalog.operations.find(
      (entry: { core: { id: string } }) => entry.core.id === operation,
    );
    const response = await api.post("/api/lab/execute", {
      headers: { Origin: origin },
      data: {
        id: crypto.randomUUID(),
        operation,
        version: descriptor.lab.version,
        input,
        parameters,
        retain_result: retained,
      },
    });
    expect(response.ok()).toBe(true);
    return response.json();
  }
  const decodedChannel = await coreProbe(
    "image.decode",
    { type: "bytes", base64: channelData.preview_png_base64 },
    [],
  );
  const channelPixel = await coreProbe(
    "image.pixels.read",
    { type: "artifact", id: decodedChannel.result.id },
    [
      { name: "x", value: 0 },
      { name: "y", value: 0 },
    ],
    false,
  );
  expect([...Buffer.from(channelPixel.inline.base64, "base64")]).toEqual([80]);
  await lab.getByRole("button", { name: "Invert", exact: true }).click();
  await expect(
    lab.getByLabel("Result provenance", { exact: true }),
  ).toHaveValue(/image.invert/);
  await lab.getByRole("button", { name: "Read pixel", exact: true }).click();
  await expect(
    lab.getByText("Core pixel (0, 0) · red, green, blue: 235, 175, 205", {
      exact: true,
    }),
  ).toBeVisible();
  await lab
    .getByRole("button", { name: "Compare with original", exact: true })
    .click();
  await expect(
    lab.getByLabel("Structured result content", { exact: true }),
  ).toHaveValue(/"differing_pixels": 4/);
  await expect(
    lab.getByLabel("Structured result content", { exact: true }),
  ).toHaveValue(/"equal": false/);
  await lab
    .getByRole("button", { name: "1. image.decode · image", exact: true })
    .click();
  await expect(
    lab.getByRole("button", { name: "1. image.decode · image", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(
    lab.getByLabel("Result provenance", { exact: true }),
  ).toHaveValue(/image.decode/);
  await lab
    .getByRole("button", { name: "Compute histogram", exact: true })
    .click();
  await expect(
    lab.getByRole("region", { name: "Exact histogram", exact: true }),
  ).toBeVisible();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  expect(
    await (await api.get("/api/workspace/media?path=sample.png")).body(),
  ).toEqual(bytes);
  expect(external).toEqual([]);
  expect(errors).toEqual([]);
});

test("preserves native palette indices until explicit Core expansion", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Inspect indexed.png", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Open visual Lab", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Decode image in Core", exact: true })
    .click();
  await page.getByLabel("Pixel X", { exact: true }).fill("1");
  await page.getByRole("button", { name: "Read pixel", exact: true }).click();
  await expect(
    page.getByText("Core pixel (1, 0) · index: 1", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Expand palette to RGBA", exact: true })
    .click();
  await expect(
    page
      .getByRole("combobox", { name: "Analysis channel", exact: true })
      .locator("option"),
  ).toHaveCount(4);
  await page.getByLabel("Pixel X", { exact: true }).fill("1");
  await page.getByRole("button", { name: "Read pixel", exact: true }).click();
  await expect(
    page.getByText(
      "Core pixel (1, 0) · red, green, blue, alpha: 40, 50, 60, 128",
      { exact: true },
    ),
  ).toBeVisible();
});

test("rejects invalid and over-limit images without freezing or losing byte inspection", async ({
  page,
}) => {
  for (const name of ["malformed.png", "over-limit.png"]) {
    await page
      .getByRole("button", { name: `Inspect ${name}`, exact: true })
      .click();
    await page
      .getByRole("button", { name: "Open visual Lab", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Decode image in Core", exact: true })
      .click();
    await expect(
      page
        .getByRole("region", { name: "Visual Lab", exact: true })
        .getByRole("alert"),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Decode image in Core", exact: true }),
    ).toBeEnabled();
    await expect(
      page.getByLabel("Hexadecimal rows", { exact: true }),
    ).toBeVisible();
  }
});
