import type { ElectronApplication, Page } from "@playwright/test";

export async function createWorkspace(
  application: ElectronApplication,
  page: Page,
  title: string,
) {
  await page
    .getByRole("textbox", { name: "Workspace name", exact: true })
    .fill(title);
  try {
    await page
      .getByRole("button", { name: "Create workspace", exact: true })
      .click();
  } catch (error) {
    // Content-free diagnostics preserve the original failed action and deadline.
    const [windows, renderer] = await Promise.all([
      application
        .evaluate(({ BrowserWindow }) =>
          BrowserWindow.getAllWindows().map((window) => ({
            visible: window.isVisible(),
            focused: window.isFocused(),
            minimized: window.isMinimized(),
          })),
        )
        .catch(() => null),
      page
        .evaluate(() => ({
          visibility: document.visibilityState,
          ready: document.readyState,
          fonts: document.fonts.status,
          paint: performance
            .getEntriesByType("paint")
            .map(({ name, startTime }) => ({ name, startTime })),
        }))
        .catch(() => null),
    ]);
    console.error("native_workspace_startup", { windows, renderer });
    throw error;
  }
}
