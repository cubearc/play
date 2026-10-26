import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { parseCandidates } from "./candidate-api";
import { parseResult } from "./lab-api";
import { CandidateControls, StageCandidate } from "./StageCandidate";
import { id, wire } from "./test/lab-fixture";

const key = "b".repeat(64);
const empty = { schema_version: 1, candidate: null, history: [], error: null };
function context() {
  return {
    schema_version: 1,
    challenges: [{ key, id: "local", title: "Local" }],
    active: {
      definition: {
        key,
        store: "source",
        path: "rabbit.cube",
        manifest: {
          id: "local",
          title: "Local",
          description: "",
          stages: [
            {
              id: "one",
              title: "Bytes stage",
              instructions: "Inspect",
              steps: [],
              hints: [],
              artifacts: [],
              answer_type: "bytes",
            },
            {
              id: "two",
              title: "Text stage",
              instructions: "Record",
              steps: [],
              hints: [],
              artifacts: [],
              answer_type: "text",
            },
          ],
        },
      },
      stage: "one",
      revision: 1,
      artifacts: [],
      progress: [],
    },
  };
}
function record(revision = 1) {
  return {
    revision,
    result_id: id,
    representation: "bytes",
    byte_size: 3,
    sha256: "a".repeat(64),
    artifact_sha256: "b".repeat(64),
    artifact_operation: "encoding.base64.decode",
    core_version: "0.18.0",
    selection: { kind: "whole" },
    status: "private_unverified",
  };
}
afterEach(() => vi.unstubAllGlobals());
it("associates whole bytes, explicitly selects a structured field and retains private revision history", async () => {
  let current: Record<string, unknown> = empty;
  const writes: Record<string, unknown>[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) => {
      if (url.endsWith("/challenges")) return Response.json(context());
      if (options?.method === "PUT") {
        writes.push(JSON.parse(String(options.body)));
        const selected = record(writes.length);
        current = {
          schema_version: 1,
          candidate: selected,
          history: [selected],
          error: null,
        };
      }
      return Response.json(current);
    }),
  );
  const result = parseResult(
    wire("image.channel.extract", {
      result: {
        ...wire().result,
        representation: "structured",
        metadata: null,
      },
    }),
  );
  render(<CandidateControls result={result} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Use result as stage candidate" }),
  );
  await screen.findByText("No private candidate selected for this stage.");
  fireEvent.click(
    screen.getByRole("button", { name: "Associate private candidate" }),
  );
  await screen.findByText(/Private candidate revision 1/);
  expect(writes[0]?.selection).toEqual({ kind: "whole" });
  fireEvent.change(screen.getByLabelText("Candidate stage"), {
    target: { value: "two" },
  });
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Associate private candidate" }),
    ).toBeEnabled(),
  );
  fireEvent.change(screen.getByLabelText("Candidate selection"), {
    target: { value: "field" },
  });
  fireEvent.change(screen.getByLabelText("JSON field pointer"), {
    target: { value: "/preview_png_base64" },
  });
  fireEvent.change(screen.getByLabelText("Field interpretation"), {
    target: { value: "base64_bytes" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Associate private candidate" }),
  );
  await screen.findByText(/Private candidate revision 2/);
  expect(writes[1]?.selection).toEqual({
    kind: "field",
    pointer: "/preview_png_base64",
    conversion: "base64_bytes",
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Reload candidate revision" }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Associate private candidate" }),
    ).toBeEnabled(),
  );
});
it("shows missing-artifact recovery and refreshes a persisted stage candidate", async () => {
  const selected = record();
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      Response.json({
        schema_version: 1,
        candidate: selected,
        history: [selected],
        error: "Artifact missing.",
      }),
    ),
  );
  render(<StageCandidate challengeKey={key} stage="one" />);
  await screen.findByText(/Private candidate revision 1/);
  expect(screen.getByRole("alert")).toHaveTextContent("Artifact missing");
  fireEvent.click(
    screen.getByRole("button", { name: "Refresh stage candidate" }),
  );
  await screen.findByText(/Artifact missing/);
});
it("does not offer image coercion and handles absent challenges, stale writes and malformed records", async () => {
  for (const value of [
    {},
    { ...empty, schema_version: 2 },
    { ...empty, history: null },
    { ...empty, candidate: { ...record(), status: "verified" } },
    { ...empty, candidate: { ...record(), byte_size: 65537 } },
  ])
    expect(() => parseCandidates(value)).toThrow();
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json({ schema_version: 1, challenges: [], active: null }),
    )
    .mockResolvedValueOnce(Response.json(context()))
    .mockResolvedValueOnce(Response.json(empty))
    .mockResolvedValueOnce(
      Response.json(
        { message: "Changed in another request." },
        { status: 409 },
      ),
    );
  vi.stubGlobal("fetch", fetcher);
  const view = render(<CandidateControls result={parseResult(wire())} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Use result as stage candidate" }),
  );
  await screen.findByText(/Import or activate/);
  fireEvent.click(
    screen.getByRole("button", { name: "Use result as stage candidate" }),
  );
  await screen.findByText("No private candidate selected for this stage.");
  expect(
    screen.getByRole("button", { name: "Associate private candidate" }),
  ).toBeDisabled();
  view.unmount();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, options?: RequestInit) =>
      url.endsWith("/challenges")
        ? Response.json(context())
        : options?.method === "PUT"
          ? Response.json(
              { message: "Changed in another request." },
              { status: 409 },
            )
          : Response.json(empty),
    ),
  );
  render(
    <CandidateControls
      result={parseResult(
        wire("decode", {
          result: { ...wire().result, representation: "bytes", metadata: null },
        }),
      )}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Use result as stage candidate" }),
  );
  await screen.findByText("No private candidate selected for this stage.");
  fireEvent.click(
    screen.getByRole("button", { name: "Associate private candidate" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Changed in another request",
  );
});
it("reports local read failures", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockRejectedValue(new Error("Local request failed.")),
  );
  render(<StageCandidate challengeKey={key} stage="one" />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Local request failed",
  );
});
