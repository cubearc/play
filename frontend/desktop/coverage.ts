// Test-driver lifecycle: collect before ordinary close or an explicit app.exit.
export function captureBeforeClose(
  application: { close(): Promise<void> },
  capture: () => Promise<void>,
): () => Promise<void> {
  let captured: Promise<void> | undefined;
  const flush = () => (captured ??= Promise.resolve().then(capture));
  const close = application.close.bind(application);
  let closed: Promise<void> | undefined;
  application.close = () =>
    (closed ??= (async () => {
      try {
        await flush();
      } finally {
        await close();
      }
    })());
  return flush;
}
