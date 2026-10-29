import { constants } from "node:fs";
import { open } from "node:fs/promises";

export async function readImport(
  path: string,
  maximum: number,
  code: string,
): Promise<Buffer> {
  const handle = await open(path, constants.O_RDONLY | constants.O_NONBLOCK);
  try {
    const metadata = await handle.stat();
    if (!metadata.isFile()) throw new Error("invalid_file");
    if (metadata.size > maximum) throw new Error(code);
    const buffer = Buffer.alloc(maximum + 1);
    let count = 0;
    while (count < buffer.length) {
      const result = await handle.read(
        buffer,
        count,
        buffer.length - count,
        null,
      );
      if (!result.bytesRead) break;
      count += result.bytesRead;
    }
    if (count > maximum) throw new Error(code);
    return buffer.subarray(0, count);
  } finally {
    await handle.close();
  }
}
