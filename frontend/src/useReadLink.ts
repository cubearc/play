import { useEffect, useState } from "react";
import { readLink } from "./capability";

export function useReadLink(uri: string | null) {
  const [generation, setGeneration] = useState(0);
  const [result, setResult] = useState<{
    uri: string;
    url: string | null;
    error: string | null;
    generation: number;
  } | null>(null);
  useEffect(() => {
    if (!uri) return;
    const controller = new AbortController();
    readLink(uri, controller.signal).then(
      (url) => {
        if (!controller.signal.aborted)
          setResult({ uri, url, error: null, generation });
      },
      (error) => {
        if (!controller.signal.aborted)
          setResult({
            uri,
            generation,
            url: null,
            error:
              error instanceof Error
                ? error.message
                : "Private media link unavailable.",
          });
      },
    );
    return () => controller.abort();
  }, [uri, generation]);
  const value =
    result?.uri === uri && result.generation === generation
      ? result
      : { uri, url: null, error: null };
  return { ...value, renew: () => setGeneration((value) => value + 1) };
}
