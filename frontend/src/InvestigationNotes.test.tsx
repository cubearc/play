import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { InvestigationNotes } from "./InvestigationNotes";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
it("coalesces rapid drafts and serializes edits made while a save is in flight", async () => {
  let release: (value: Response) => void = () => {};
  const sent: { revision: number; body: string }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (_url, options) => {
      if (options.method !== "PUT") return Response.json([]);
      const value = JSON.parse(options.body);
      sent.push(value);
      if (sent.length === 1)
        return new Promise<Response>((resolve) => {
          release = resolve;
        });
      return Response.json({ revision: 2 });
    }),
  );
  render(
    <InvestigationNotes target={{ store: "source", path: "sample.png" }} />,
  );
  await screen.findByText("Notes saved locally");
  vi.useFakeTimers();
  fireEvent.click(screen.getByRole("button", { name: "New note" }));
  fireEvent.change(screen.getByLabelText("Note title"), {
    target: { value: "Private note" },
  });
  fireEvent.change(screen.getByLabelText("Note body"), {
    target: { value: "First" },
  });
  await act(() => vi.advanceTimersByTimeAsync(401));
  fireEvent.change(screen.getByLabelText("Note body"), {
    target: { value: "Newer draft" },
  });
  await act(async () => release(Response.json({ revision: 1 })));
  expect(sent).toHaveLength(2);
  expect(sent[1]?.body).toBe("Newer draft");
  expect(sent[1]?.revision).toBe(1);
  expect(screen.getByLabelText("Note body")).toHaveValue("Newer draft");
  expect(screen.getByText("Notes saved locally")).toBeInTheDocument();
  fireEvent.click(
    screen.getByRole("button", {
      name: "Associate with selected file or investigation",
    }),
  );
  await act(() => vi.advanceTimersByTimeAsync(401));
  expect(
    screen.getByText("Associated with: source/sample.png"),
  ).toBeInTheDocument();
});
it("preserves a stale draft after conflict and requires explicit reload before replacement", async () => {
  const note = {
    id: crypto.randomUUID(),
    title: "Existing",
    body: "Stored",
    revision: 2,
    target: null,
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async (_url, options) =>
      options.method === "PUT"
        ? Response.json({ message: "Changed in another tab." }, { status: 409 })
        : Response.json([note]),
    ),
  );
  render(<InvestigationNotes target={null} />);
  fireEvent.click(await screen.findByRole("button", { name: "Existing" }));
  fireEvent.change(screen.getByLabelText("Note body"), {
    target: { value: "<script>private draft</script>" },
  });
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Changed in another tab",
  );
  expect(screen.getByLabelText("Note body")).toHaveValue(
    "<script>private draft</script>",
  );
  fireEvent.click(
    screen.getByRole("button", {
      name: "Reload notes and discard local draft",
    }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("alert")).not.toBeInTheDocument(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Existing" }));
  expect(screen.getByLabelText("Note body")).toHaveValue("Stored");
});
it("shows storage read failures and does not claim a saved note", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(
        Response.json({ message: "Storage unavailable." }, { status: 503 }),
      ),
  );
  render(<InvestigationNotes target={null} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Storage unavailable",
  );
});
