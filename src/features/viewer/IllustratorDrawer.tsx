import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Markdown } from "../../components/Markdown";
import { Button } from "../../components/Button";
import { IconButton } from "../../components/IconButton";
import { Textarea } from "../../components/Textarea";
import { AlertIcon, InfoIcon } from "../../app/Icons";
import { illustratorApi } from "../../ipc/illustrator";
import { inTauri, IpcError } from "../../ipc/client";
import { useAppSettings } from "../settings/useAppSettings";
import { useToast } from "../../components/useToast";
import type {
  DetailLevel,
  Scope,
  GenerateStarted,
  Citation,
} from "../../ipc/types.gen";
import { useStream } from "./useStream";
import styles from "./IllustratorDrawer.module.css";

const WHOLE = { t: "whole" as const };
const LEVELS: DetailLevel[] = ["simple", "standard", "detailed"];

function asCitations(value: unknown): Citation[] {
  return Array.isArray(value) ? (value as Citation[]) : [];
}

/** Session boundary: the newest turn known when this Live session opened.
 * `createdAt` is second-precision (`now_iso8601`), so UUIDv7 `id`s break
 * same-second ties (they sort lexicographically in creation order). */
export type LiveAnchor = { at: string; id: string };

/** Split a thread into turns that predated this Live session (`past`) and
 * turns asked during it (`live`). ISO-8601 strings compare
 * lexicographically. */
export function partitionLiveMessages<T extends { createdAt: string; id: string }>(
  messages: T[],
  anchor: LiveAnchor | null,
): { past: T[]; live: T[] } {
  if (!anchor) return { past: [], live: [...messages] };
  return {
    past: messages.filter(
      (m) => m.createdAt < anchor.at || (m.createdAt === anchor.at && m.id <= anchor.id),
    ),
    live: messages.filter(
      (m) => m.createdAt > anchor.at || (m.createdAt === anchor.at && m.id > anchor.id),
    ),
  };
}

/** One chat turn in message format (design.md §Studio, compact for the
 * drawer): user turns are end-aligned bubbles, assistant turns stay open on
 * the canvas with a role label. Text is selectable and copyable; citations
 * appear only when the backend resolved some. */
function MessageBubble({
  role,
  content,
  citations,
  streaming,
  onCitation,
}: {
  role: "user" | "assistant";
  content: string;
  citations?: unknown;
  streaming?: boolean;
  onCitation?: (c: Citation) => void;
}) {
  const { t } = useTranslation();
  const toast = useToast();
  const cites = asCitations(citations);

  async function copy() {
    try {
      await navigator.clipboard.writeText(content);
      toast.push({ tone: "success", message: t("illustrator.copied") });
    } catch {
      toast.push({ tone: "error", message: t("errors.internal") });
    }
  }

  return (
    <div
      className={styles.msg}
      data-role={role}
      data-streaming={streaming || undefined}
    >
      <div className={styles.msgHead}>
        <span className={styles.role}>
          {role === "assistant"
            ? t("studio.role.assistant")
            : t("studio.role.user")}
        </span>
        <button
          type="button"
          className={styles.copyBtn}
          onClick={() => void copy()}
        >
          {t("illustrator.copy")}
        </button>
      </div>
      <Markdown>{content}</Markdown>
      {cites.length > 0 ? (
        <div className={styles.citations}>
          {cites.map((c, i) => (
            <button
              type="button"
              key={`${c.sourceId}-${i}`}
              className={styles.citation}
              disabled={!onCitation}
              onClick={() => onCitation?.(c)}
            >
              [{i + 1}] {c.sourceName}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/** Live Illustrator (docs/05 §4). Standalone from Studio — its own Q&A thread,
 * never fed into a Studio tab. On open it explains the whole source at the
 * detail level chosen in Settings; the two other levels are offered as
 * one-tap rewrites *below* the explanation, but only while the reader has not
 * asked a follow-up yet. When it was just enabled it waits for one explicit
 * tap before spending tokens (`autoRun={false}`). */
export function IllustratorDrawer({
  projectId,
  sourceId,
  locator,
  visionSupported,
  autoRun = true,
  onStarted,
  onCitation,
}: {
  projectId: string;
  sourceId: string | null;
  locator: unknown;
  visionSupported: boolean;
  autoRun?: boolean;
  onStarted?: () => void;
  onCitation?: (c: Citation) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const { settings } = useAppSettings();
  const defaultLevel = (settings?.illustrator.defaultLevel ?? "standard") as DetailLevel;
  // null → follow the Settings default; set → the reader picked a rewrite level.
  const [levelOverride, setLevelOverride] = useState<DetailLevel | null>(null);
  const level = levelOverride ?? defaultLevel;
  const [scope, setScope] = useState<Scope>("source");
  const [gen, setGen] = useState<GenerateStarted | null>(null);
  const [genErr, setGenErr] = useState<string | null>(null);
  const [askStreamId, setAskStreamId] = useState<string | null>(null);
  const [askErr, setAskErr] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [historyOpen, setHistoryOpen] = useState(false);
  const [started, setStarted] = useState(autoRun);
  // Session boundary for "past vs live": the newest message known when
  // this source was opened. Anything newer arrived during this Live
  // session and stays expanded; anything older is past.
  const [anchor, setAnchor] = useState<LiveAnchor | null>(null);
  const bodyRef = useRef<HTMLDivElement>(null);

  // One Q&A thread per source (FR-L4) — questions survive page moves.
  const thread = useQuery({
    queryKey: ["ill-thread", projectId, sourceId],
    enabled: inTauri && !!sourceId,
    queryFn: () =>
      illustratorApi.getOrCreateThread(projectId, sourceId!, { t: "whole" }),
  });
  const genStream = useStream(gen?.streamId ?? null);
  const askStream = useStream(askStreamId);
  const errorText = (error: unknown) =>
    error instanceof IpcError
      ? t([`errors.${error.code}`, "errors.internal"])
      : t("errors.internal");

  // Debounced whole-source explanation trigger (FR-L2). Waits for `started` so a
  // just-enabled panel does not fire until the reader asks. Re-fires when the
  // reader switches detail level.
  const debounce = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => {
    if (!inTauri || !sourceId || !genStream.ready || !started) return;
    setGen(null);
    setGenErr(null);
    setAskErr(null);
    clearTimeout(debounce.current);
    debounce.current = setTimeout(() => {
      illustratorApi
        .generate({ projectId, sourceId, locator: WHOLE, level })
        .then(setGen)
        .catch((e) => setGenErr(errorText(e)));
    }, 300);
    return () => clearTimeout(debounce.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, sourceId, level, genStream.ready, started]);

  // A new source starts again from the Settings default, with a fresh
  // past/live boundary and a closed history.
  useEffect(() => {
    setLevelOverride(null);
    setAnchor(null);
    setHistoryOpen(false);
  }, [sourceId, projectId]);

  useEffect(() => {
    if (
      askStreamId &&
      !askStream.streaming &&
      (askStream.done || askStream.error)
    ) {
      if (askStream.error) {
        setAskErr(
          t([`errors.${askStream.errorCode ?? "internal"}`, "errors.internal"]),
        );
      }
      void qc.invalidateQueries({ queryKey: ["ill-thread", projectId, sourceId] });
      setAskStreamId(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [askStream.streaming, askStream.done, askStream.error]);

  // Capture the session boundary once the thread first loads. Later
  // invalidations (after our own questions) must not move it.
  useEffect(() => {
    if (anchor !== null || !thread.data) return;
    const last = thread.data.messages.at(-1);
    setAnchor({ at: last?.createdAt ?? "", id: last?.id ?? "" });
  }, [thread.data, anchor]);

  function begin() {
    setStarted(true);
    onStarted?.();
  }

  function regenerate() {
    if (!sourceId || !genStream.ready) return;
    setGen(null);
    setGenErr(null);
    setAskErr(null);
    illustratorApi
      .generate({ projectId, sourceId, locator: WHOLE, level, force: true })
      .then(setGen)
      .catch((e) => setGenErr(errorText(e)));
  }

  async function send() {
    if (!text.trim() || !thread.data || !askStream.ready) return;
    setAskErr(null);
    try {
      const sid = await illustratorApi.ask({
        projectId,
        threadId: thread.data.id,
        text: text.trim(),
        scope,
        locator,
      });
      setText("");
      setAskStreamId(sid);
    } catch (e) {
      setAskErr(errorText(e));
    }
  }

  function stop() {
    if (gen?.streamId) void illustratorApi.cancel(gen.streamId);
    if (askStreamId) void illustratorApi.cancel(askStreamId);
  }

  const handoff = useMutation({
    mutationFn: () => {
      if (!thread.data) throw new Error("no-live-thread");
      return illustratorApi.importToStudio({
        projectId,
        threadId: thread.data.id,
        mode: "new_tab",
        targetTabId: null,
      });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["studio-tabs", projectId] });
      toast.push({ tone: "success", message: t("illustrator.handoffSuccess") });
    },
    onError: () =>
      toast.push({ tone: "error", message: t("illustrator.handoffFailed") }),
  });

  const cached = gen?.cached ?? null;
  const explanation = cached?.content ?? genStream.text;
  const generating = !cached && genStream.streaming;
  const allMessages = (thread.data?.messages ?? []).filter((m) => m.content !== "");
  // Past = turns that already existed when this Live session opened.
  // Live = turns asked during this session — always expanded, never hidden
  // inside the "past" fold.
  const { past: pastMessages, live: liveMessages } = partitionLiveMessages(
    allMessages,
    anchor,
  );
  const streaming = genStream.streaming || askStream.streaming;
  const streamError =
    genErr ??
    (genStream.error
      ? t([`errors.${genStream.errorCode ?? "internal"}`, "errors.internal"])
      : null) ??
    askErr;
  // The rewrite buttons appear only while the reader has not asked anything
  // *in this session* — older past turns must not take them away.
  const canRewrite =
    started &&
    !!explanation &&
    !streamError &&
    !streaming &&
    liveMessages.length === 0 &&
    !askStreamId;

  // Follow the live conversation as it grows, but never yank a text
  // selection the reader is making (selectable answers).
  const liveCount = liveMessages.length;
  useEffect(() => {
    const el = bodyRef.current;
    if (!el || (liveCount === 0 && !askStreamId)) return;
    if (document.getSelection()?.toString()) return;
    el.scrollTop = el.scrollHeight;
  }, [liveCount, askStreamId, askStream.text]);

  async function copyText(content: string) {
    try {
      await navigator.clipboard.writeText(content);
      toast.push({ tone: "success", message: t("illustrator.copied") });
    } catch {
      toast.push({ tone: "error", message: t("errors.internal") });
    }
  }

  if (!sourceId) {
    return <div className={styles.empty}>{t("illustrator.openADocument")}</div>;
  }

  return (
    <div className={styles.drawer}>
      <header className={styles.head}>
        <div className={styles.headTop}>
          <span className={styles.title}>{t("viewer.illustrator")}</span>
          {started && explanation ? (
            <IconButton label={t("illustrator.regenerate")} size="sm" onClick={regenerate}>
              ↻
            </IconButton>
          ) : null}
        </div>
        <span className={styles.pos}>
          {t("illustrator.wholeSource")} · {t(`illustrator.${level}`)}
        </span>
      </header>

      {!visionSupported ? (
        <p className={styles.banner}>
          <AlertIcon size={13} />
          {t("illustrator.visionOff")}
        </p>
      ) : null}

      <div ref={bodyRef} className={styles.body} aria-live="polite">
        {!started ? (
          <div className={styles.startCard}>
            <p>{t("illustrator.overviewHint")}</p>
            <Button variant="primary" onClick={begin}>
              {t("viewer.illustratorStart")}
            </Button>
          </div>
        ) : streamError ? (
          <p className={styles.error}>
            <AlertIcon size={14} /> {streamError}
          </p>
        ) : explanation ? (
          <div className={styles.explanation}>
            <div className={styles.explanationHead}>
              {cached ? (
                <p className={styles.cachedNote}>
                  <InfoIcon size={13} />
                  {t("illustrator.cached")} ·{" "}
                  {new Date(cached.createdAt).toLocaleString()}
                </p>
              ) : (
                <span />
              )}
              <button
                type="button"
                className={styles.copyBtn}
                onClick={() => void copyText(explanation)}
              >
                {t("illustrator.copy")}
              </button>
            </div>
            <Markdown>{explanation}</Markdown>
          </div>
        ) : generating ? (
          <p className={styles.thinking}>{t("common.loading")}…</p>
        ) : (
          <p className={styles.thinking}>{t("illustrator.overviewHint")}</p>
        )}

        {canRewrite ? (
          <div className={styles.levelSwap}>
            <span className={styles.levelSwapLabel}>{t("illustrator.rewriteAs")}</span>
            {LEVELS.filter((l) => l !== level).map((l) => (
              <Button
                key={l}
                size="sm"
                variant="quiet"
                onClick={() => setLevelOverride(l)}
              >
                {t(`illustrator.${l}`)}
              </Button>
            ))}
          </div>
        ) : null}

        {liveMessages.length > 0 ? (
          <section
            className={styles.live}
            aria-label={t("illustrator.live")}
          >
            <p className={styles.liveLabel}>{t("illustrator.live")}</p>
            {liveMessages.map((m) => (
              <MessageBubble
                key={m.id}
                role={m.role === "user" ? "user" : "assistant"}
                content={m.content}
                citations={m.citations}
                onCitation={onCitation}
              />
            ))}
          </section>
        ) : null}

        {askStreamId ? (
          <MessageBubble
            role="assistant"
            content={askStream.text || "…"}
            citations={askStream.citations}
            streaming
            onCitation={onCitation}
          />
        ) : null}

        {pastMessages.length > 0 ? (
          <div className={styles.history}>
            <button
              type="button"
              className={styles.historyToggle}
              aria-expanded={historyOpen}
              onClick={() => setHistoryOpen((v) => !v)}
            >
              {t("illustrator.past", {
                count: Math.ceil(pastMessages.length / 2),
              })}
            </button>
            {historyOpen
              ? pastMessages.map((m) => (
                  <MessageBubble
                    key={m.id}
                    role={m.role === "user" ? "user" : "assistant"}
                    content={m.content}
                    citations={m.citations}
                    onCitation={onCitation}
                  />
                ))
              : null}
          </div>
        ) : null}
      </div>

      <footer className={styles.composer}>
        <div className={styles.askRow}>
          <Textarea
            value={text}
            placeholder={t("illustrator.ask")}
            rows={2}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                void send();
              }
            }}
          />
          <IconButton
            label={t("illustrator.send")}
            onClick={() => void send()}
            disabled={!text.trim() || !askStream.ready}
          >
            ↑
          </IconButton>
        </div>
        <div className={styles.composerMeta}>
          <select
            className={styles.scopeSel}
            value={scope}
            onChange={(e) => setScope(e.target.value as Scope)}
            aria-label={t("illustrator.scope")}
          >
            <option value="page">{t("illustrator.scopePage")}</option>
            <option value="source">{t("illustrator.scopeSource")}</option>
            <option value="project">{t("illustrator.scopeProject")}</option>
          </select>
          {streaming ? (
            <Button size="sm" variant="quiet" onClick={stop}>
              {t("illustrator.stop")}
            </Button>
          ) : null}
          <Button
            size="sm"
            variant="quiet"
            disabled={
              !thread.data ||
              (!explanation &&
                pastMessages.length === 0 &&
                liveMessages.length === 0) ||
              streaming
            }
            loading={handoff.isPending}
            onClick={() => handoff.mutate()}
          >
            {t("illustrator.toStudio")}
          </Button>
        </div>
      </footer>
    </div>
  );
}
