import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Markdown } from "../../components/Markdown";
import { Button } from "../../components/Button";
import { IconButton } from "../../components/IconButton";
import { Textarea } from "../../components/Textarea";
import { Tabs } from "../../components/Tabs";
import { AlertIcon, InfoIcon } from "../../app/Icons";
import { illustratorApi } from "../../ipc/illustrator";
import { inTauri } from "../../ipc/client";
import type { DetailLevel, Scope, GenerateStarted } from "../../ipc/types.gen";
import { useStream } from "./useStream";
import styles from "./IllustratorDrawer.module.css";

export function IllustratorDrawer({
  projectId,
  sourceId,
  locator,
  position,
  visionSupported,
}: {
  projectId: string;
  sourceId: string | null;
  locator: unknown;
  position?: string;
  visionSupported: boolean;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [level, setLevel] = useState<DetailLevel>("standard");
  const [scope, setScope] = useState<Scope>("source");
  const [gen, setGen] = useState<GenerateStarted | null>(null);
  const [genErr, setGenErr] = useState<string | null>(null);
  const [askStreamId, setAskStreamId] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [historyOpen, setHistoryOpen] = useState(false);

  const locatorKey = useMemo(() => JSON.stringify(locator ?? {}), [locator]);

  const thread = useQuery({
    queryKey: ["ill-thread", projectId, sourceId, locatorKey],
    enabled: inTauri && !!sourceId,
    queryFn: () => illustratorApi.getOrCreateThread(projectId, sourceId!, locator),
  });

  // Debounced (300ms) explanation trigger on locator / level change (FR-L2).
  const debounce = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => {
    if (!inTauri || !sourceId) return;
    setGen(null);
    setGenErr(null);
    clearTimeout(debounce.current);
    debounce.current = setTimeout(() => {
      illustratorApi
        .generate({ projectId, sourceId, locator, level })
        .then(setGen)
        .catch((e) => setGenErr(e?.message ?? String(e)));
    }, 300);
    return () => clearTimeout(debounce.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, sourceId, locatorKey, level]);

  const genStream = useStream(gen?.streamId ?? null);
  const askStream = useStream(askStreamId);

  // After a question finishes, reload the thread and clear the transient stream.
  useEffect(() => {
    if (askStreamId && !askStream.streaming && (askStream.done || askStream.error)) {
      void qc.invalidateQueries({ queryKey: ["ill-thread", projectId, sourceId, locatorKey] });
      setAskStreamId(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [askStream.streaming, askStream.done, askStream.error]);

  async function send() {
    if (!text.trim() || !thread.data) return;
    const sid = await illustratorApi.ask({
      projectId,
      threadId: thread.data.id,
      text: text.trim(),
      scope,
    });
    setText("");
    setAskStreamId(sid);
  }

  function stop() {
    if (gen?.streamId) void illustratorApi.cancel(gen.streamId);
    if (askStreamId) void illustratorApi.cancel(askStreamId);
  }

  const cached = gen?.cached ?? null;
  const explanation = cached?.content ?? genStream.text;
  const generating = !cached && genStream.streaming;
  const pastMessages = thread.data?.messages ?? [];

  if (!sourceId) {
    return <div className={styles.empty}>{t("illustrator.openADocument")}</div>;
  }

  return (
    <div className={styles.drawer}>
      <header className={styles.head}>
        <Tabs
          label={t("illustrator.detail")}
          variant="segmented"
          value={level}
          onChange={(v) => setLevel(v as DetailLevel)}
          items={[
            { id: "simple", label: t("illustrator.simple") },
            { id: "standard", label: t("illustrator.standard") },
            { id: "detailed", label: t("illustrator.detailed") },
          ]}
        />
        <div className={styles.headRow}>
          <span className={styles.pos}>{position ?? ""}</span>
          <Button
            size="sm"
            variant="quiet"
            onClick={() => {
              setGen(null);
              illustratorApi
                .generate({ projectId, sourceId, locator, level, force: true })
                .then(setGen)
                .catch((e) => setGenErr(e?.message ?? String(e)));
            }}
          >
            ↻ {t("illustrator.regenerate")}
          </Button>
        </div>
      </header>

      {!visionSupported ? (
        <p className={styles.banner}>
          <AlertIcon size={13} />
          {t("illustrator.visionOff")}
        </p>
      ) : null}

      <div className={styles.body} aria-live="polite">
        {genErr ? (
          <p className={styles.error}>
            <AlertIcon size={14} /> {genErr}
          </p>
        ) : explanation ? (
          <>
            {cached ? (
              <p className={styles.cachedNote}>
                <InfoIcon size={13} />
                {t("illustrator.cached")} · {new Date(cached.createdAt).toLocaleString()}
              </p>
            ) : null}
            <Markdown>{explanation}</Markdown>
          </>
        ) : generating ? (
          <p className={styles.thinking}>{t("common.loading")}…</p>
        ) : (
          <p className={styles.thinking}>{t("illustrator.willExplain")}</p>
        )}

        {pastMessages.length > 0 ? (
          <div className={styles.history}>
            <button
              type="button"
              className={styles.historyToggle}
              aria-expanded={historyOpen}
              onClick={() => setHistoryOpen((v) => !v)}
            >
              {t("illustrator.past", { count: Math.ceil(pastMessages.length / 2) })}
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
        <div className={styles.composerTop}>
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
          {genStream.streaming || askStream.streaming ? (
            <Button size="sm" variant="quiet" onClick={stop}>
              {t("illustrator.stop")}
            </Button>
          ) : null}
        </div>
        <div className={styles.composerRow}>
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
          <IconButton label={t("illustrator.send")} onClick={() => void send()} disabled={!text.trim()}>
            ↑
          </IconButton>
        </div>
        <div className={styles.composerRow}>
          <Button
            size="sm"
            variant="quiet"
            disabled={!thread.data || pastMessages.length === 0}
            onClick={() =>
              thread.data &&
              void illustratorApi.importToStudio({
                projectId,
                threadId: thread.data.id,
                mode: "new_tab",
                targetTabId: null,
              })
            }
          >
            {t("illustrator.toStudio")}
          </Button>
        </div>
      </footer>
    </div>
  );
}
