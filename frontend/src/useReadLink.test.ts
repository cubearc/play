import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { readLink } from "./capability";
import { useReadLink } from "./useReadLink";

it("renews expired media access explicitly and never displays a stale URL for a different resource", async () => {
  vi.mocked(readLink).mockRejectedValueOnce(new Error("Media lease expired"));
  const { result, rerender, unmount } = renderHook(
    ({ uri }) => useReadLink(uri),
    { initialProps: { uri: "/api/workspace/media?path=one.png" } },
  );
  await waitFor(() => expect(result.current.error).toBe("Media lease expired"));
  act(() => result.current.renew());
  await waitFor(() =>
    expect(result.current.url).toContain("path=one.png&read_cap="),
  );
  let complete: ((url: string) => void) | undefined;
  vi.mocked(readLink).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
  );
  rerender({ uri: "/api/workspace/media?path=other.png" });
  expect(result.current.url).toBeNull();
  unmount();
  await act(async () =>
    complete?.("/api/workspace/media?path=other.png&read_cap=late"),
  );
});
