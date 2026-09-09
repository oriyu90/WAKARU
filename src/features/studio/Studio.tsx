import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button";
import { IconButton } from "../../components/IconButton";
import { EmptyState } from "../../components/EmptyState";
import { Input } from "../../components/Input";
import { Markdown } from "../../components/Markdown";
import { Textarea } from "../../components/Textarea";
import { CloseIcon } from "../../app/Icons";
import { studioApi } from "../../ipc/studio";
import { pickSaveDir } from "../../ipc/fileModifier";
import { inTauri } from "../../ipc/client";
import { useToast } from "../../components/useToast";
import { listen } from "@tauri-apps/api/event";
import type {
  Artifact,
  ChatMessage,
  Citation,
  Source,
  StudioTab,
} from "../../ipc/types.gen";
import styles from "./Studio.module.css";

type StudioToolCall = { id: string; name: string; arguments: unknown };
type StudioDelta = { tabId: string; kind: string; text: string };
type StudioToolEvent = { tabId: string; name: string; state: "running" | "complete" };

export function Studio({
  projectId,
  projectName,
  sources,
  onCitation,
}: {
  projectId: string;
  projectName: string;
  sources: Source[];
  onCitation: (c: Citation) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();

  const [activeId, setActiveId] = useState("");
  const [text, setText] = useState("");
  const [scope, setScope] = useState("project");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [provisional, setProvisional] = useState("");
  const [runningTool, setRunningTool] = useState("");
  const composerRef = useRef<HTMLTextAreaElement>(null);
  const messagesRef = useRef<HTMLDivElement>(null);

  const tabs = useQuery({
    queryKey: ["studio-tabs", projectId],
    queryFn: () => studioApi.listTabs(projectId),
    enabled: inTauri,
  });
  const artifacts = useQuery({
    queryKey: ["studio-artifacts", projectId],
    queryFn: () => studioApi.listArtifacts(projectId),
    enabled: inTauri,
  });

  const refreshTabs = () => qc.invalidateQueries({ queryKey: ["studio-tabs", projectId] });
  const refreshArtifacts = () =>
    qc.invalidateQueries({ queryKey: ["studio-artifacts", projectId] });

  const rows = tabs.data ?? [];
  const active = rows.find((tb) => tb.id === activeId) ?? rows[0];
  const activeTabId = active?.id ?? "";

  // Keep the local scope selector in step with the active tab.
  useEffect(() => {
    if (active) setScope(active.scope);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active?.id, active?.scope]);

  // Pin `activeId` to a tab that actually exists. After a refetch (send, close,
  // create) or a reload, `activeId` can be "" or point at a closed tab; without
  // this the selection silently falls back to the first tab on every render.
  useEffect(() => {
    if (active && active.id !== activeId) setActiveId(active.id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active?.id]);

  const messages = useMemo(() => active?.messages ?? [], [active]);
  const last = messages.at(-1);
  const awaitingApproval = last?.status === "pending_approval";
  const needsContinue = last?.status === "needs_continue";
  const locked = awaitingApproval || needsContinue;

  const create = useMutation({
    mutationFn: () => studioApi.createTab(projectId),
    onSuccess: async (tab) => {
      setActiveId(tab.id);
      await refreshTabs();
      composerRef.current?.focus();
    },
  });
  const close = useMutation({
    mutationFn: (id: string) => studioApi.closeTab(projectId, id),
    onSuccess: async () => {
      setActiveId("");
      await refreshTabs();
    },
  });
  const rename = useMutation({
    mutationFn: (id: string) => studioApi.renameTab(projectId, id, title.trim()),
    onSuccess: async () => {
      setRenaming(null);
      await refreshTabs();
    },
  });
  const send = useMutation({
    mutationFn: (payload: string) => studioApi.send(projectId, activeTabId, payload, scope),
    onMutate: () => {
      setProvisional("");
      setRunningTool("");
    },
    onSuccess: async () => {
      // Clear the streamed placeholder *before* the refetch resolves, so the
      // persisted message never renders alongside its own provisional copy.
      setText("");
      setProvisional("");
      setRunningTool("");
      await Promise.all([refreshTabs(), refreshArtifacts()]);
      composerRef.current?.focus();
    },
    onError: (e) => toast.push({ tone: "error", message: (e as Error).message }),
  });

  useEffect(() => {
    if (!inTauri || !activeTabId) return;
    let disposed = false;
    const cleanups: Array<() => void> = [];
    void Promise.all([
      listen<StudioDelta>("studio://delta", ({ payload }) => {
        if (payload.tabId === activeTabId && payload.kind === "text") {
          setProvisional((value) => value + payload.text);
        }
      }),
      listen<StudioToolEvent>("studio://tool", ({ payload }) => {
        if (payload.tabId !== activeTabId) return;
        setRunningTool(payload.state === "running" ? payload.name : "");
      }),
    ]).then((unlisten) => {
      if (disposed) unlisten.forEach((fn) => fn());
      else cleanups.push(...unlisten);
    });
    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
    };
  }, [activeTabId]);
  const resolveTool = useMutation({
    mutationFn: (approved: boolean) => studioApi.resolveTool(projectId, activeTabId, approved),
    onMutate: () => { setProvisional(""); setRunningTool(""); },
    onSuccess: async () => {
      setProvisional("");
      setRunningTool("");
      await Promise.all([refreshTabs(), refreshArtifacts()]);
    },
    onError: (e) => toast.push({ tone: "error", message: (e as Error).message }),
  });
  const streaming = send.isPending || resolveTool.isPending;
  const importArtifact = useMutation({
    mutationFn: (id: string) => studioApi.importArtifact(projectId, id),
    onSuccess: async () => {
      await Promise.all([
        refreshArtifacts(),
        qc.invalidateQueries({ queryKey: ["sources", projectId] }),
      ]);
    },
    onError: () => toast.push({ tone: "error", message: t("studio.importFailed") }),
  });
  const download = useMutation({
    mutationFn: async (id: string) => {
      const dir = await pickSaveDir();
      if (!dir) throw new Error("cancelled");
      return studioApi.downloadArtifact(projectId, id, dir);
    },
    onSuccess: (path) => toast.push({ tone: "success", message: t("studio.downloaded", { path }) }),
    onError: (e) => {
      if (e instanceof Error && e.message === "cancelled") return;
      toast.push({ tone: "error", message: t("errors.io") });
    },
  });

  // Follow the conversation as it grows.
  useEffect(() => {
    const node = messagesRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [messages.length, send.isPending]);

  function submit() {
    if (!text.trim() || !active || locked || send.isPending) return;
    send.mutate(text.trim());
  }

  function mention(tab: StudioTab) {
    setText((v) => `${v}${v && !v.endsWith(" ") ? " " : ""}@${tab.title} `);
    composerRef.current?.focus();
  }

  if (!inTauri) return <div className={styles.emptyPane} />;

  if (!rows.length) {
    return (
      <div className={styles.emptyPane}>
        <EmptyState
          title={t("studio.empty")}
          body={t("studio.emptyBody")}
          actions={
            <Button variant="primary" loading={create.isPending} onClick={() => create.mutate()}>
              {t("studio.newTab")}
            </Button>
          }
        />
      </div>
    );
  }

  return (
    <div className={styles.studio}>
      <aside className={styles.rail} aria-label={t("studio.tabs")}>
        <ul className={styles.railList}>
          {rows.map((tab) => (
            <li key={tab.id}>
              {renaming === tab.id ? (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (title.trim()) rename.mutate(tab.id);
                  }}
                >
                  <Input
                    autoFocus
                    aria-label={t("studio.renameLabel")}
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                    onBlur={() => setRenaming(null)}
                  />
                </form>
              ) : (
                <div className={styles.railRow} data-active={tab.id === active?.id}>
                  <button
                    type="button"
                    className={styles.railButton}
                    aria-current={tab.id === active?.id}
                    onClick={() => setActiveId(tab.id)}
                    onDoubleClick={() => {
                      setRenaming(tab.id);
                      setTitle(tab.title);
                    }}
                  >
                    <span className={styles.railName}>{tab.title}</span>
                    <span className={`${styles.railCount} u-mono-nums`}>
                      {t("studio.messageCount", { count: tab.messages.length })}
                    </span>
                  </button>
                  <IconButton
                    label={t("studio.rename")}
                    size="sm"
                    onClick={() => {
                      setRenaming(tab.id);
                      setTitle(tab.title);
                    }}
                  >
                    ✎
                  </IconButton>
                  <IconButton
                    label={t("studio.closeTab")}
                    size="sm"
                    onClick={() => close.mutate(tab.id)}
                  >
                    <CloseIcon size={13} />
                  </IconButton>
                </div>
              )}
            </li>
          ))}
        </ul>
        <Button variant="quiet" size="sm" loading={create.isPending} onClick={() => create.mutate()}>
          + {t("studio.newTab")}
        </Button>
      </aside>

      <main className={styles.conversation}>
        <div ref={messagesRef} className={styles.messages} aria-live="polite">
          {messages.map((m) => (
            <MessageRow
              key={m.id}
              message={m}
              onCitation={onCitation}
              onAllow={() => resolveTool.mutate(true)}
              onDeny={() => resolveTool.mutate(false)}
              onContinue={() => send.mutate("")}
              busy={resolveTool.isPending || send.isPending}
            />
          ))}
          {streaming && provisional ? (
            <div className={styles.streaming} aria-label={t("studio.streaming")}><Markdown>{provisional}</Markdown></div>
          ) : null}
          {streaming ? <p className={styles.thinking}>{runningTool ? t("studio.runningTool", { name: runningTool }) : t("studio.thinking")}</p> : null}
        </div>

        {/* AC-6-11: the working directory is always on screen, even when the
            workspace column is collapsed on a narrow window. */}
        <p className={styles.wsBanner}>
          {t("studio.workspace")}:{" "}
          <code>{t("studio.workspacePath", { project: projectName })}</code>
        </p>

        <form
          className={styles.composer}
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          {rows.length > 1 ? (
            <div className={styles.mentions}>
              {rows
                .filter((tb) => tb.id !== active?.id)
                .map((tb) => (
                  <button
                    type="button"
                    key={tb.id}
                    className={styles.mention}
                    onClick={() => mention(tb)}
                  >
                    @{tb.title}
                  </button>
                ))}
            </div>
          ) : null}

          <div className={styles.composerRow}>
            <Textarea
              ref={composerRef}
              rows={2}
              aria-label={t("studio.message")}
              placeholder={t("studio.messagePlaceholder")}
              value={text}
              disabled={locked}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  submit();
                }
              }}
            />
            {send.isPending ? (
              <Button variant="quiet" size="sm" onClick={() => studioApi.cancel(activeTabId)}>
                {t("studio.stop")}
              </Button>
            ) : (
              <IconButton
                label={t("studio.send")}
                onClick={submit}
                disabled={!text.trim() || locked}
              >
                ↑
              </IconButton>
            )}
          </div>

          <div className={styles.composerBar}>
            <select
              className={styles.scopeSel}
              aria-label={t("studio.scope")}
              value={scope}
              onChange={(e) => setScope(e.target.value)}
            >
              <option value="project">{t("illustrator.scopeProject")}</option>
              {sources
                .filter((s) => s.status === "ready")
                .map((s) => (
                  <option key={s.id} value={`source:${s.id}`}>
                    {t("illustrator.scopeSource")}: {s.originalName}
                  </option>
                ))}
            </select>
            <span className={styles.hint}>{t("studio.sendHint")}</span>
          </div>
        </form>
      </main>

      <aside className={styles.workspace} aria-label={t("studio.workspace")}>
        <div className={styles.wsHead}>
          <span className={styles.wsTitle}>{t("studio.workspace")}</span>
          <code className={styles.wsPath}>{t("studio.workspacePath", { project: projectName })}</code>
        </div>
        {artifacts.data?.length ? (
          <ul className={styles.artifacts}>
            {artifacts.data.map((a: Artifact) => (
              <li key={a.id} className={styles.artifact}>
                <span className={styles.artifactName}>{a.relPath}</span>
                <span className={`${styles.artifactSize} u-mono-nums`}>{a.bytes} B</span>
                <div className={styles.artifactActions}>
                  <Button
                    variant="quiet"
                    size="sm"
                    disabled={Boolean(a.importedSourceId)}
                    disabledReason={a.importedSourceId ? t("studio.alreadyImported") : undefined}
                    loading={importArtifact.isPending && importArtifact.variables === a.id}
                    onClick={() => importArtifact.mutate(a.id)}
                  >
                    {a.importedSourceId ? t("studio.imported") : t("studio.addSource")}
                  </Button>
                  <Button
                    variant="quiet"
                    size="sm"
                    loading={download.isPending && download.variables === a.id}
                    onClick={() => download.mutate(a.id)}
                  >
                    {t("studio.download")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p className={styles.wsEmpty}>{t("studio.workspaceEmpty")}</p>
        )}
      </aside>
    </div>
  );
}

function MessageRow({
  message,
  onCitation,
  onAllow,
  onDeny,
  onContinue,
  busy,
}: {
  message: ChatMessage;
  onCitation: (c: Citation) => void;
  onAllow: () => void;
  onDeny: () => void;
  onContinue: () => void;
  busy: boolean;
}) {
  const { t } = useTranslation();
  const roleKey = `studio.role.${message.role}`;
  const role = t(roleKey) === roleKey ? message.role : t(roleKey);
  const calls = (message.toolCalls ?? []) as StudioToolCall[];

  if (message.role === "tool") {
    return (
      <article className={styles.message} data-role="tool">
        <span className={styles.role}>{t("studio.toolResult")}</span>
        <pre className={styles.toolOut}>{message.content}</pre>
      </article>
    );
  }

  return (
    <article className={styles.message} data-role={message.role}>
      <span className={styles.role}>{role}</span>
      {message.content ? <Markdown>{message.content}</Markdown> : null}

      {message.status === "pending_approval" && calls.length ? (
        <section className={styles.approval} aria-label={t("studio.approvalTitle")}>
          <strong>{t("studio.approvalTitle")}</strong>
          <p>{t("studio.approvalBody")}</p>
          {calls.map((c, i) => (
            <div key={c.id ?? i}>
              <p className={styles.approvalTool}>{c.name}</p>
              {/* Full arguments, never abbreviated (AC-7-2). */}
              <pre className={styles.approvalArgs}>
                {typeof c.arguments === "string"
                  ? c.arguments
                  : JSON.stringify(c.arguments, null, 2)}
              </pre>
            </div>
          ))}
          <div className={styles.approvalActions}>
            <Button variant="primary" onClick={onAllow} loading={busy}>
              {t("studio.allow")}
            </Button>
            <Button variant="danger" onClick={onDeny} loading={busy}>
              {t("studio.deny")}
            </Button>
          </div>
        </section>
      ) : null}

      {message.status === "needs_continue" ? (
        <div className={styles.continue}>
          <span className={styles.continueHint}>{t("studio.continueHint")}</span>
          <Button variant="secondary" size="sm" onClick={onContinue} loading={busy}>
            {t("studio.continue")}
          </Button>
        </div>
      ) : null}

      {message.citations && (message.citations as Citation[]).length ? (
        <div className={styles.citations}>
          {(message.citations as Citation[]).map((c, i) => (
            <button
              type="button"
              key={`${c.sourceId}-${i}`}
              className={styles.citation}
              onClick={() => onCitation(c)}
            >
              [{i + 1}] {c.sourceName}
            </button>
          ))}
        </div>
      ) : null}
    </article>
  );
}
