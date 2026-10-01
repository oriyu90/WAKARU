import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { VisualPreview } from "../../ipc/types.gen";
import { Button } from "../../components/Button";
import { useToast } from "../../components/useToast";
import styles from "./InteractivePreview.module.css";

/** Max persisted operation state per figure (plan §6 — safe JSON only). */
const STATE_MAX = 16 * 1024;

function stateKey(id: string) {
  return `wakaru.visual-state.${id}`;
}

function loadState(id: string): string {
  try {
    return localStorage.getItem(stateKey(id)) ?? "{}";
  } catch {
    return "{}";
  }
}

/** Build the isolated document. The iframe runs on an opaque origin
 * (`sandbox="allow-scripts"` with no `allow-same-origin`), receives no IPC
 * bridge and no project asset URL, and is fenced by a meta CSP: no network,
 * no frames, no forms. Math is expected as generated SVG (no CDN KaTeX). */
function buildSrcDoc(visual: VisualPreview, token: string, savedState: string) {
  const safeState = savedState.length <= STATE_MAX ? savedState : "{}";
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; media-src data: blob:; font-src data:; connect-src 'none'; frame-src 'none'; object-src 'none'; worker-src 'none'; base-uri 'none'; form-action 'none';"><style>html,body{margin:0;padding:0}body{font:0.875rem/1.5 system-ui,sans-serif;padding:0.75rem;box-sizing:border-box}svg{max-width:100%;height:auto}canvas{max-width:100%}</style><style>${visual.css}</style></head><body>${visual.html}<script>window.__WAKARU_TOKEN__=${JSON.stringify(token)};window.__WAKARU_STATE__=${safeState};window.__WAKARU_DATA__=${visual.dataJson == null ? "{}" : JSON.stringify(visual.dataJson).slice(0, 16384)};(function(){var t=window.__WAKARU_TOKEN__;window.__wakaruNotify=function(s){try{var raw=JSON.stringify(s);if(raw.length>${STATE_MAX})return;parent.postMessage({token:t,state:JSON.parse(raw)},"*");}catch(e){}};})();<\/script><script>${visual.js}</script></body></html>`;
}

/** Shared figure renderer for Studio and Live (plan §4.3). Same artifact,
 * same chrome: loading, failure + retry, copy, expand, reset. Narrow
 * containers (drawer) get a compact layout via ResizeObserver. */
export function InteractivePreview({
  visual,
  onRetry,
}: {
  visual: VisualPreview;
  onRetry?: () => void;
}) {
  const { t } = useTranslation();
  const toast = useToast();
  const frameRef = useRef<HTMLIFrameElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [compact, setCompact] = useState(false);
  const [nonce, setNonce] = useState(0);
  // One token per mount: state notifications must echo it or they are
  // ignored. Never forwarded anywhere else.
  const token = useId().replace(/[^a-zA-Z0-9]/g, "");
  const [savedState, setSavedState] = useState(() => loadState(visual.id));

  const srcDoc = useMemo(
    () => buildSrcDoc(visual, token, savedState),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [visual.id, nonce],
  );

  useEffect(() => {
    setLoading(true);
    setFailed(false);
  }, [visual.id, nonce]);

  // Compact layout for narrow drawers; wide Studio panes keep full chrome.
  useEffect(() => {
    const node = boxRef.current;
    if (!node || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      setCompact(w > 0 && w < 26 * 16);
    });
    ro.observe(node);
    return () => ro.disconnect();
  }, []);

  // Accept only schema-checked state notes from our own frame instance.
  useEffect(() => {
    function onMessage(event: MessageEvent) {
      if (event.source !== frameRef.current?.contentWindow) return;
      const data = event.data as { token?: unknown; state?: unknown } | null;
      if (!data || typeof data !== "object" || data.token !== token) return;
      try {
        const raw = JSON.stringify(data.state ?? {});
        if (raw.length > STATE_MAX) return;
        setSavedState(raw);
        try {
          localStorage.setItem(stateKey(visual.id), raw);
        } catch {
          // Private mode etc. — the figure still works for this session.
        }
      } catch {
        // Unserializable payloads are dropped, never executed.
      }
    }
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [token, visual.id]);

  async function copySource() {
    try {
      await navigator.clipboard.writeText(visual.html);
      toast.push({ tone: "success", message: t("visual.copied") });
    } catch {
      toast.push({ tone: "error", message: t("errors.internal") });
    }
  }

  function reset() {
    try {
      localStorage.removeItem(stateKey(visual.id));
    } catch {
      // Ignore persistence failures; the in-memory state still resets.
    }
    setSavedState("{}");
    setNonce((n) => n + 1);
  }

  if (failed) {
    return (
      <section className={styles.figure} aria-label={visual.title}>
        <p className={styles.error}>{t("visual.failed")}</p>
        {onRetry ? (
          <Button variant="secondary" size="sm" onClick={onRetry}>
            {t("visual.retry")}
          </Button>
        ) : null}
      </section>
    );
  }

  return (
    <section
      ref={boxRef}
      className={styles.figure}
      data-compact={compact || undefined}
      data-expanded={expanded || undefined}
      aria-label={visual.title}
    >
      <div className={styles.head}>
        <span className={styles.title}>{visual.title}</span>
        <span className={styles.actions}>
          <button type="button" className={styles.tool} onClick={() => void copySource()}>
            {t("visual.copy")}
          </button>
          <button type="button" className={styles.tool} onClick={reset}>
            {t("visual.reset")}
          </button>
          <button
            type="button"
            className={styles.tool}
            aria-expanded={expanded}
            onClick={() => setExpanded((v) => !v)}
          >
            {expanded ? t("visual.shrink") : t("visual.expand")}
          </button>
        </span>
      </div>
      <div className={styles.frameBox} style={{ aspectRatio: visual.aspectRatio.replace(":", " / ") }}>
        {loading ? <p className={styles.loading}>{t("visual.loading")}</p> : null}
        <iframe
          ref={frameRef}
          key={`${visual.id}-${nonce}`}
          title={visual.title}
          className={styles.frame}
          sandbox="allow-scripts"
          referrerPolicy="no-referrer"
          srcDoc={srcDoc}
          onLoad={() => setLoading(false)}
          onError={() => {
            setLoading(false);
            setFailed(true);
          }}
        />
      </div>
    </section>
  );
}
