export const APP_URL = "cube-play://app/index.html";

const resources = new Map([
  [APP_URL, "index.html"],
  ["cube-play://app/app.js", "app.js"],
  ["cube-play://app/app.css", "app.css"],
  ["cube-play://app/app.ttf", "app.ttf"],
]);

export function resourceName(url: string): string | null {
  // Compare the original wire URL, before a URL parser can normalize traversal.
  return resources.get(url) ?? null;
}

interface Frame {
  readonly url: string;
}

interface Sender {
  readonly mainFrame: Frame;
}

export function ownsFrame(
  event: { readonly sender: Sender; readonly senderFrame: Frame | null },
  trusted: Sender,
): boolean {
  return (
    event.sender === trusted &&
    event.senderFrame === trusted.mainFrame &&
    event.senderFrame.url === APP_URL
  );
}
