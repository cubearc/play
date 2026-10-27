import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import AxeBuilder from "@axe-core/playwright";
import { expect, openSection, test } from "./session";

test.skip(
  !process.env.CUBE_PLAY_DOCKER_TEST_URL,
  "This suite requires the real two-container runtime.",
);
test("executes real Cube commands through xterm and closes the process session", async ({
  page,
}) => {
  // Real CLI, resizing and the full accessibility audit share this test budget.
  test.setTimeout(60_000);
  const url = process.env.CUBE_PLAY_DOCKER_TEST_URL;
  if (!url) throw new Error("Real runner session unavailable");
  await page.goto(url);
  await openSection(page, "Terminal");
  await page
    .getByRole("button", { name: "Start terminal", exact: true })
    .click();
  await expect(
    page.getByText("Connected to Cube Runner", { exact: true }),
  ).toBeVisible();
  const input = page.getByRole("textbox", {
    name: "Cube terminal input",
    exact: true,
  });
  await input.pressSequentially("cube encode base64 hello");
  await input.press("Enter");
  await expect(
    page.getByRole("region", { name: "Cube terminal output" }),
  ).toContainText("aGVsbG8=");
  await input.pressSequentially("cube nonexistent-command");
  await input.press("Enter");
  await expect(
    page.getByRole("region", { name: "Cube terminal output" }),
  ).toContainText("unrecognized subcommand");
  const size = page.getByText(/Terminal size: \d+/);
  await expect(size).toContainText("×");
  const before = await size.textContent();
  await page.setViewportSize({
    width: test.info().project.name === "mobile" ? 430 : 900,
    height: 800,
  });
  await expect(size).not.toHaveText(before ?? "");
  const accessibility = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(accessibility.violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "End session", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Start terminal", exact: true }),
  ).toBeVisible();
});

test("publishes a real CLI capture explicitly without rerunning and keeps its replay gaps", async ({
  page,
}) => {
  const url = process.env.CUBE_PLAY_DOCKER_TEST_URL;
  if (!url) throw new Error("Real runner session unavailable");
  await page.goto(url);
  await openSection(page, "Terminal");
  await page
    .getByRole("button", { name: "Start terminal", exact: true })
    .click();
  await expect(
    page.getByText("Connected to Cube Runner", { exact: true }),
  ).toBeVisible();
  const input = page.getByRole("textbox", {
    name: "Cube terminal input",
    exact: true,
  });
  await input.pressSequentially("cube --lab-capture text reverse --text CUBE");
  await input.press("Enter");
  await expect(
    page.getByRole("region", { name: "Cube terminal output" }),
  ).toContainText("EBUC");
  await page.getByRole("button", { name: "Refresh staged captures" }).click();
  await expect(
    page.getByRole("button", { name: "Send to Lab", exact: true }),
  ).toHaveCount(1);
  await page.getByRole("button", { name: "Send to Lab", exact: true }).click();
  const output = page.getByRole("region", {
    name: "Selected operation outputs",
    exact: true,
  });
  await expect(
    output.getByLabel("Exact text output", { exact: true }),
  ).toHaveValue("EBUC");
  await expect(
    output.getByRole("region", { name: "Replay availability", exact: true }),
  ).toContainText("Original inputs");
  const identity = await output.locator("code").first().textContent();
  await page.getByRole("button", { name: "Send to Lab", exact: true }).click();
  await expect(output.locator("code").first()).toHaveText(identity ?? "");
  if (!identity) throw new Error("The published artifact identity is missing");
  const title = `Terminal capture ${test.info().project.name} ${identity.slice(0, 8)}`;
  await output.getByLabel("Artifact name", { exact: true }).fill(title);
  await output
    .getByRole("button", { name: "Save named artifact", exact: true })
    .click();
  await expect(output.getByText(/Artifact saved locally/)).toBeVisible();
  await page.getByRole("button", { name: "End session", exact: true }).click();
  await openSection(page, "Artifacts");
  await page
    .getByRole("button", { name: "Refresh artifact library", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: title,
      exact: true,
    })
    .click();
  await expect(
    page.getByLabel("Stored result provenance", { exact: true }),
  ).toHaveValue(/text.reverse/);
});

test("publishes exact CLI byte, image and report captures through their trusted viewers", async ({
  page,
  api,
}) => {
  test.setTimeout(90_000);
  const link = process.env.CUBE_PLAY_DOCKER_TEST_URL;
  if (!link) throw new Error("Real runner session unavailable");
  const name = `capture-${randomUUID()}.png`;
  const imported = await api.post(`/api/workspace/import?name=${name}`, {
    headers: {
      Origin: new URL(link).origin,
      "Content-Type": "application/octet-stream",
    },
    data: await readFile("../tests/fixtures/sample.png"),
  });
  expect(imported.status()).toBe(201);
  await page.goto(link);
  await openSection(page, "Terminal");
  await page
    .getByRole("button", { name: "Start terminal", exact: true })
    .click();
  await expect(
    page.getByText("Connected to Cube Runner", { exact: true }),
  ).toBeVisible();
  const input = page.getByRole("textbox", {
    name: "Cube terminal input",
    exact: true,
  });
  const publication = page.getByRole("region", {
    name: "Terminal to Lab",
    exact: true,
  });
  let previous: string[] = [];
  for (const [operation, command] of [
    ["encoding.hex.decode", "decode hex --text 00ff80 --output raw"],
    [
      "image.invert",
      `image invert /workspace/data/imports/${name} --channels red --output /workspace/output/${name}`,
    ],
    [
      "image.histogram",
      `image histogram /workspace/data/imports/${name} --channels red --max-colors 4 --json`,
    ],
  ]) {
    // Exercise real key events without exceeding the existing 200-frame/s limit.
    await input.pressSequentially(`cube --lab-capture ${command}`, {
      delay: 10,
    });
    await input.press("Enter");
    await expect
      .poll(async () => {
        await publication
          .getByRole("button", { name: "Refresh staged captures", exact: true })
          .click();
        return publication.locator("li code").count();
      })
      .toBe(previous.length + 1);
    const ids = await publication.locator("li code").allTextContents();
    const id = ids.find((id) => !previous.includes(id));
    if (!id)
      throw new Error("The completed CLI command did not stage a new capture");
    previous = ids;
    await publication
      .getByRole("listitem")
      .filter({ hasText: id })
      .getByRole("button", { name: "Send to Lab", exact: true })
      .click();
    const output = publication.getByRole("region", {
      name: "Selected operation outputs",
      exact: true,
    });
    await expect(
      output.getByRole("heading", {
        name: `Result · ${operation}`,
        exact: true,
      }),
    ).toBeVisible();
    if (operation === "encoding.hex.decode") {
      await expect(
        output.getByLabel("Base64 byte output", { exact: true }),
      ).toHaveValue("AP+A");
    } else if (operation === "image.invert") {
      const image = output.getByRole("region", {
        name: "Exact raster viewer",
        exact: true,
      });
      await expect(image).toContainText("2 × 2");
      await expect(image.getByRole("table")).toContainText("235");
    } else {
      await expect(
        output.getByRole("region", { name: "Exact histogram", exact: true }),
      ).toContainText("4 pixels");
    }
    await expect(
      output.getByRole("region", { name: "Replay availability", exact: true }),
    ).toContainText("Original inputs");
    await output
      .getByLabel("Artifact name", { exact: true })
      .fill(`CLI ${operation} ${id}`);
    await output
      .getByRole("button", { name: "Save named artifact", exact: true })
      .click();
    await expect(output.getByText(/Artifact saved locally/)).toBeVisible();
  }
  await page.getByRole("button", { name: "End session", exact: true }).click();
});
