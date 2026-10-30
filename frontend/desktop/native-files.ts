interface NativeFiles {
  engine: string;
  cli: string;
  worker?: string;
  terminal?: string;
  terminal_cli?: string;
}

// Preparation, integrity verification and startup share this exact layout.
export function nativeFiles(): NativeFiles {
  const suffix = process.platform === "win32" ? ".exe" : "";
  const files: NativeFiles = {
    engine: `play-engine${suffix}`,
    cli: `cube${suffix}`,
  };
  if (process.platform === "darwin") {
    files.worker = "Worker.app/Contents/MacOS/play-engine";
    files.terminal = "Terminal.app/Contents/MacOS/play-engine";
    files.terminal_cli = "Terminal.app/Contents/MacOS/cube";
  } else if (process.platform === "linux") {
    // The same verified binary starts a separate, restricted worker process.
    files.worker = "play-worker";
    files.terminal = "play-terminal";
  }
  return files;
}
