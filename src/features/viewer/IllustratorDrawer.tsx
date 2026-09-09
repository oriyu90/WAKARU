import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Markdown } from "../../components/Markdown";
import { Button } from "../../components/Button";
import { IconButton } from "../../components/IconButton";
import { Textarea } from "../../components/Textarea";
import { AlertIcon, InfoIcon } from "../../app/Icons";
import { illustratorApi } from "../../ipc/illustrator";
import { inTauri, IpcError } from "../../ipc/client";
import { useAppSettings } from "../settings/useAppSettings";
import type { DetailLevel, Scope, GenerateStarted } from "../../ipc/types.gen";
import { useStream } from "./useStream";
import styles from "./IllustratorDrawer.module.css";

const WHOLE = { t: "whole" as const };
const LEVELS: DetailLevel[] = ["simple", "standard", "detailed"];

/** Live Illustrator (docs/05 §4). Standalone from Studio — its own Q&A thread,
 * never fed into a Studio tab. On open it explains the whole source at the
 * detail level chosen in Settings; the two other levels are offered as
 * one-tap rewrites *below* the explanation, but only while the reader has not
 * asked a follow-up yet. When it was just enabled it waits for one explicit
 * tap before spending tokens (`autoRun={false}`). */
export function IllustratorDrawer({
  projectId,
  sourceId,
  visionSupported,
  autoRun = true,
  onStarted,
}: {
  projectId: string;
  sourceId: string | null;
  visionSupported: boolean;
  autoRun?: boolean;
  onStarted?: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
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

  // A new source starts again from the Settings default.
  useEffect(() => {
    setLevelOverride(null);
  }, [sourceId]);

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

  const cached = gen?.cached ?? null;
  const explanation = cached?.content ?? genStream.text;
  const generating = !cached && genStream.streaming;
  const pastMessages = thread.data?.messages ?? [];
  const streaming = genStream.streaming || askStream.streaming;
  const streamError =
    genErr ??
    (genStream.error
      ? t([`errors.${genStream.errorCode ?? "internal"}`, "errors.internal"])
      : null) ??
    askErr;
  // The rewrite buttons appear only right after the first auto explanation —
  // once the reader has asked anything, the explanation is part of a
  // conversation and silently swapping it would be confusing.
  const canRewrite =
    started &&
    !!explanation &&
    !streamError &&
    !streaming &&
    pastMessages.length === 0 &&
    !askStreamId;

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

      <div className={styles.body} aria-live="polite">
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
          <>
            {cached ? (
              <p className={styles.cachedNote}>
                <InfoIcon size={13} />
                {t("illustrator.cached")} ·{" "}
                {new Date(cached.createdAt).toLocaleString()}
              </p>
            ) : null}
            <Markdown>{explanation}</Markdown>
          </>
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
                  <div key={m.id} className={styles.msg} data-role={m.role}>
                    <Markdown>{m.content}</Markdown>
                  </div>
                ))
              : null}
          </div>
        ) : null}

        {askStreamId ? (
          <div className={styles.msg} data-role="assistant">
            <Markdown>{askStream.text || "…"}</Markdown>
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
        </div>
      </footer>
    </div>
  );
}
