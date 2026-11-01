//! Observe transient queue states in their renderer, using the actual controls.
import type { Page } from "@playwright/test";

export async function exerciseQueueControls(page: Page) {
  return page.evaluate(async () => {
    const cell = (name: string) => {
      const element = document.querySelector<HTMLElement>(
        `article[aria-label="${name} block"]`,
      );
      if (!element) throw new Error(`Missing ${name} cell`);
      return element;
    };
    const first = cell("First clue"),
      second = cell("Second clue");
    const status = (element: HTMLElement) =>
      element.querySelector('[role="status"]')?.textContent?.trim() ?? "";
    const button = (element: HTMLElement, name: string) => {
      const result = [
        ...element.querySelectorAll<HTMLElement>("vscode-button,button"),
      ].find((button) => button.textContent?.trim() === name);
      if (!result) throw new Error(`Missing ${name} control`);
      return result;
    };
    const enabled = (element: HTMLElement) =>
      !element.hasAttribute("disabled") &&
      element.getAttribute("aria-disabled") !== "true";
    const require = (condition: boolean, detail: string) => {
      if (!condition)
        throw new Error(`${detail}: ${status(first)} / ${status(second)}`);
    };
    const until = (condition: () => boolean) =>
      new Promise<void>((resolve, reject) => {
        const observer = new MutationObserver(() => {
          if (condition()) {
            clearTimeout(timer);
            observer.disconnect();
            resolve();
          }
        });
        const timer = setTimeout(() => {
          observer.disconnect();
          reject(
            new Error(
              `Queue state unavailable: ${status(first)} / ${status(second)}`,
            ),
          );
        }, 5000);
        observer.observe(document.body, {
          subtree: true,
          childList: true,
          characterData: true,
          attributes: true,
        });
        if (condition()) {
          clearTimeout(timer);
          observer.disconnect();
          resolve();
        }
      });
    button(first, "Run cell").click();
    await until(
      () => status(first) === "Running…" || status(first).startsWith("Failed:"),
    );
    require(status(first) === "Running…", "First cell did not run");
    require(enabled(
      button(second, "Run cell"),
    ), "Unrelated Trail was disabled");
    button(second, "Run cell").click();
    await until(
      () =>
        status(second) === "Queued…" || status(second).startsWith("Failed:"),
    );
    require(status(second) === "Queued…", "Second cell did not queue");
    require(status(first) === "Running…", "Queued cell changed the active Run");
    const observed = [status(first), status(second)];
    button(second, "Cancel").click();
    await until(() => status(second) === "Cancelled: user_cancelled");
    require(status(first) ===
      "Running…", "Queued cancellation changed the active Run");
    observed.push(status(first), status(second));
    button(first, "Cancel").click();
    await until(() => status(first) === "Cancelled: user_cancelled");
    observed.push(status(first), status(second));
    return observed;
  });
}
