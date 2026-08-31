import { useEffect, useRef, useState } from "react";
import { inTauri } from "../../ipc/client";
import type {
  StreamDelta,
  StreamDone,
  StreamError,
  StreamCitations,
} from "../../ipc/types.gen";

export type StreamState = {
  text: string;
  reasoning: string;
  citations: unknown[];
  streaming: boolean;
  error: string | null;
  done: StreamDone | null;
};

const EMPTY: StreamState = {
  text: "",
  reasoning: "",
  citations: [],
  streaming: false,
  error: null,
  done: null,
};

/** Subscribe to `stream://*` for one `streamId`. Pass `null` to reset. */
export function useStream(streamId: string | null): StreamState {
  const [state, setState] = useState<StreamState>(EMPTY);
  const idRef = useRef<string | null>(null);
  idRef.current = streamId;

  useEffect(() => {
    if (!streamId) {
      setState(EMPTY);
      return;
    }
    setState({ ...EMPTY, streaming: true });
    if (!inTauri) return;

    const unlisteners: Array<() => void> = [];
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const match = (id: string) => id === idRef.current;

      unlisteners.push(
        await listen<StreamDelta>("stream://delta", (e) => {
          if (!match(e.payload.streamId)) return;
          setState((s) =>
            e.payload.kind === "reasoning"
              ? { ...s, reasoning: s.reasoning + e.payload.text }
              : { ...s, text: s.text + e.payload.text },
          );
        }),
      );
      unlisteners.push(
        await listen<StreamCitations>("stream://citations", (e) => {
          if (!match(e.payload.streamId)) return;
          setState((s) => ({ ...s, citations: e.payload.citations as unknown[] }));
        }),
      );
      unlisteners.push(
        await listen<StreamDone>("stream://done", (e) => {
          if (!match(e.payload.streamId)) return;
          setState((s) => ({ ...s, streaming: false, done: e.payload }));
        }),
      );
      unlisteners.push(
        await listen<StreamError>("stream://error", (e) => {
          if (!match(e.payload.streamId)) return;
          setState((s) => ({
            ...s,
            streaming: false,
            error: e.payload.error.message,
          }));
        }),
      );
    })();

    return () => unlisteners.forEach((u) => u());
  }, [streamId]);

  return state;
}
