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
  errorCode: string | null;
  done: StreamDone | null;
  /** `true` once the native event listeners can safely receive a new stream. */
  ready: boolean;
};

const EMPTY: StreamState = {
  text: "",
  reasoning: "",
  citations: [],
  streaming: false,
  error: null,
  errorCode: null,
  done: null,
  ready: !inTauri,
};

/**
 * Subscribe to `stream://*` before a stream id exists, then filter events by
 * the current id. Waiting until after `invoke()` returns can lose a fast local
 * model's first delta while the Tauri listener is still being installed.
 */
export function useStream(streamId: string | null): StreamState {
  const [state, setState] = useState<StreamState>(EMPTY);
  const idRef = useRef<string | null>(null);
  // This is separate from `idRef`: an event can arrive after React renders a
  // new id but before the corresponding effect resets visible state.
  const stateIdRef = useRef<string | null>(null);
  idRef.current = streamId;

  useEffect(() => {
    if (!inTauri) return;

    const unlisteners: Array<() => void> = [];
    let active = true;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const match = (id: string) => id === idRef.current;
      const update = (id: string, apply: (previous: StreamState) => StreamState) => {
        setState((previous) => {
          const isFirstEventForStream = stateIdRef.current !== id;
          stateIdRef.current = id;
          return apply(
            isFirstEventForStream
              ? { ...EMPTY, ready: previous.ready, streaming: true }
              : previous,
          );
        });
      };

      const listeners = await Promise.all([
        listen<StreamDelta>("stream://delta", (e) => {
          if (!match(e.payload.streamId)) return;
          update(e.payload.streamId, (previous) =>
            e.payload.kind === "reasoning"
              ? { ...previous, reasoning: previous.reasoning + e.payload.text }
              : { ...previous, text: previous.text + e.payload.text },
          );
        }),
        listen<StreamCitations>("stream://citations", (e) => {
          if (!match(e.payload.streamId)) return;
          update(e.payload.streamId, (previous) => ({
            ...previous,
            citations: e.payload.citations as unknown[],
          }));
        }),
        listen<StreamDone>("stream://done", (e) => {
          if (!match(e.payload.streamId)) return;
          update(e.payload.streamId, (previous) => ({ ...previous, streaming: false, done: e.payload }));
        }),
        listen<StreamError>("stream://error", (e) => {
          if (!match(e.payload.streamId)) return;
          update(e.payload.streamId, (previous) => ({
            ...previous,
            streaming: false,
            error: e.payload.error.message,
            errorCode: e.payload.error.code,
          }));
        }),
      ]);
      if (!active) {
        listeners.forEach((unlisten) => unlisten());
        return;
      }
      unlisteners.push(...listeners);
      setState((s) => ({ ...s, ready: true }));
    })();

    return () => {
      active = false;
      unlisteners.forEach((u) => u());
    };
  }, []);

  useEffect(() => {
    if (stateIdRef.current === streamId) return;
    stateIdRef.current = streamId;
    setState((previous) => ({
      ...EMPTY,
      ready: previous.ready,
      streaming: streamId !== null,
    }));
  }, [streamId]);

  return state;
}
