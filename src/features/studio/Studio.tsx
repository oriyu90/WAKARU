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
import { aiApi } from "../../ipc/ai";
import { pickSaveDir } from "../../ipc/fileModifier";
import { inTauri, IpcError } from "../../ipc/client";
import { useToast } from "../../components/useToast";
import { listen } from "@tauri-apps/api/event";
import type {
  AiProfile,
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
type TurnPersisted = { projectId: string; tabId: string; clientRequestId: string | null; messageId: string };
type HistoryChanged = { tabId: string };
type DocProgress = { tabId: string; done: number; total: number; phase: string };

export function newClientRequestId(): string {
  try {
    if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID();
  } catch { /* fallback below */ }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

/** Pre-persist failures must restore the draft; post-persist keeps the formal turn. */
export function isPrePersistStudioError(code: string): boolean {
  return [
    "STUDIO_EMPTY_MESSAGE",
    "STUDIO_REWIND_LOCKED",
    "STUDIO_MESSAGE_NOT_FOUND",
    "STUDIO_TAB_NOT_FOUND",
    "AI_NOT_CONFIGURED",
  ].includes(code);
}

export function shouldSubmitStudioKey(key: string, shiftKey: boolean, isComposing: boolean) {
  return key === "Enter" && !shiftKey && !isComposing;
}

/** Profiles the reader can pick for a single Studio session: only ones with
 * a default model are usable as an override target. */
export function usableChatProfiles(profiles: AiProfile[]): AiProfile[] {
  return profiles.filter((p) => (p.defaultModel ?? "").trim() !== "");
}

export function Studio({
  projectId,
  projectName,
  sources,
  onCitation,
  onSourceImported,
}: {
  projectId: string;
  projectName: string;
  sources: Source[];
  onCitation: (c: Citation) => void;
  onSourceImported: (source: Source) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const aiErrorText = (error: unknown) => error instanceof IpcError
    ? t([`errors.${error.code}`, "errors.internal"])
    : t("errors.internal");

  const [activeId, setActiveId] = useState("");
  const [text, setText] = useState("");
  const [scope, setScope] = useState("project");
  // Session-only model override (a profile id, or null for the configured
  // default). Reset whenever the active conversation changes so reopening a
  // tab always falls back to the default model.
  const [modelProfileId, setModelProfileId] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [provisional, setProvisional] = useState("");
  const [runningTool, setRunningTool] = useState("");
  const [dropActive, setDropActive] = useState(false);
  const [editingTurn, setEditingTurn] = useState<{ id: string; content: string } | null>(null);
  // v1.5.0 A-2: optimistic send display keyed by clientRequestId, per tab.
  const [optimisticByTab, setOptimisticByTab] = useState<Record<string, { clientRequestId: string; text: string }>>({});
  const [sendTargetId, setSendTargetId] = useState("");
  const draftBackup = useRef<{ text: string; editing: { id: string; content: string } | null } | null>(null);
  const [liveMsg, setLiveMsg] = useState("");
  const [docProgress, setDocProgress] = useState<DocProgress | null>(null);
  const historyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const composerRef = useRef<HTMLTextAreaElement>(null);
  const messagesRef = useRef<HTMLDivElement>(null);

  // The rail: tab metadata + message counts only, never every tab's history.
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

  const rows = tabs.data ?? [];
  // No implicit fallback to the first tab: a past conversation stays closed
  // (and its history off screen) until the reader picks it.
  const activeRow = rows.find((tb) => tb.id === activeId) ?? null;
  const activeTabId = activeRow?.id ?? "";

  // The full conversation for just the active tab.
  const conversation = useQuery({
    queryKey: ["studio-tab", projectId, activeTabId],
    queryFn: () => studioApi.getTab(projectId, activeTabId),
    enabled: inTauri && !!activeTabId,
  });
  const active = conversation.data ?? activeRow;

  const refreshTabs = () =>
    Promise.all([
      qc.invalidateQueries({ queryKey: ["studio-tabs", projectId] }),
      qc.invalidateQueries({ queryKey: ["studio-tab", projectId] }),
    ]);
  const refreshArtifacts = () =>
    qc.invalidateQueries({ queryKey: ["studio-artifacts", projectId] });

  // Keep the local scope selector in step with the active tab.
  useEffect(() => {
    if (active) setScope(active.scope);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active?.id, active?.scope]);

  useEffect(() => {
    setEditingTurn(null);
    setText("");
    // Reopening (or switching) a conversation drops the session-only model
    // override and falls back to the configured default model.
    setModelProfileId(null);
  }, [activeTabId]);

  // Chat-capable connections for the session-only model selector.
  const profiles = useQuery({
    queryKey: ["ai-profiles"],
    queryFn: aiApi.listProfiles,
    enabled: inTauri,
  });
  const roleBindings = useQuery({
    queryKey: ["ai-bindings"],
    queryFn: aiApi.getRoleBindings,
    enabled: inTauri,
  });
  const chatBinding = roleBindings.data?.chat ?? null;
  const modelOptions = useMemo(
    () => usableChatProfiles(profiles.data ?? []),
    [profiles.data],
  );

  const messages = useMemo(
    () => conversation.data?.messages ?? [],
    [conversation.data],
  );
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
    mutationFn: ({ payload, replaceFrom, targetTabId, scopeSnapshot, profileSnapshot, clientRequestId }: {
      payload: string; replaceFrom?: string; targetTabId: string; scopeSnapshot: string;
      profileSnapshot?: string; clientRequestId: string;
    }) =>
      studioApi.send(
        projectId,
        targetTabId,
        payload,
        scopeSnapshot,
        replaceFrom,
        profileSnapshot,
        clientRequestId,
      ),
    onMutate: (vars) => {
      // Freeze the destination so a tab switch mid-flight never retargets.
      setSendTargetId(vars.targetTabId);
      draftBackup.current = { text: vars.payload, editing: editingTurn };
      setOptimisticByTab((prev) => ({
        ...prev,
        [vars.targetTabId]: { clientRequestId: vars.clientRequestId, text: vars.payload },
      }));
      setLiveMsg(t("studio.sending"));
      setText("");
      setProvisional("");
      setRunningTool("");
      setDocProgress(null);
      setEditingTurn(null);
    },
    onSuccess: async (_res, vars) => {
      setLiveMsg(t("studio.sent"));
      await Promise.all([refreshTabs(), refreshArtifacts()]);
      // Drop the optimistic row once the formal history converges. The
      // turn-persisted / history-changed refetch usually already did this;
      // this covers a missed event.
      setOptimisticByTab((prev) => {
        if (!prev[vars.targetTabId]) return prev;
        const next = { ...prev };
        delete next[vars.targetTabId];
        return next;
      });
      if (vars.targetTabId === activeTabId) composerRef.current?.focus();
    },
    onError: async (e, vars) => {
      const code = e instanceof IpcError ? e.code : "";
      await refreshTabs();
      if (isPrePersistStudioError(code)) {
        // Persistence never happened: restore the draft verbatim.
        setOptimisticByTab((prev) => {
          if (!prev[vars.targetTabId]) return prev;
          const next = { ...prev };
          delete next[vars.targetTabId];
          return next;
        });
        const backup = draftBackup.current;
        if (backup) {
          setText(backup.text);
          setEditingTurn(backup.editing);
        }
        draftBackup.current = null;
        setLiveMsg(t("studio.sendFailed"));
      } else {
        // Post-persist failure/cancel: keep the formal turn, show state.
        setOptimisticByTab((prev) => {
          if (!prev[vars.targetTabId]) return prev;
          const next = { ...prev };
          delete next[vars.targetTabId];
          return next;
        });
        draftBackup.current = null;
        setLiveMsg(code === "CANCELLED" || (e instanceof Error && e.message === "cancelled") ? t("studio.cancelled") : t("studio.sendFailed"));
      }
      setProvisional("");
      setRunningTool("");
      toast.push({ tone: "error", message: aiErrorText(e) });
    },
  });

  useEffect(() => {
    if (!inTauri) return;
    let disposed = false;
    const cleanups: Array<() => void> = [];
    const scheduleHistory = (tabId: string) => {
      if (historyTimer.current) clearTimeout(historyTimer.current);
      historyTimer.current = setTimeout(() => {
        void qc.invalidateQueries({ queryKey: ["studio-tabs", projectId] });
        void qc.invalidateQueries({ queryKey: ["studio-tab", projectId, tabId] });
        void qc.invalidateQueries({ queryKey: ["studio-artifacts", projectId] });
      }, 150);
    };
    void Promise.all([
      listen<StudioDelta>("studio://delta", ({ payload }) => {
        if (payload.tabId === activeTabId && payload.kind === "text") {
          setProvisional((value) => value + payload.text);
        }
      }),
      listen<StudioToolEvent>("studio://tool", ({ payload }) => {
        if (payload.tabId !== activeTabId) return;
        setRunningTool(payload.state === "running" ? payload.name : "");
        if (payload.state === "complete") scheduleHistory(payload.tabId);
      }),
      listen<TurnPersisted>("studio://turn-persisted", ({ payload }) => {
        if (payload.projectId !== projectId) return;
        scheduleHistory(payload.tabId);
        // Reconcile the optimistic row once the formal turn is refetched.
        setTimeout(() => {
          setOptimisticByTab((prev) => {
            const opt = prev[payload.tabId];
            if (!opt) return prev;
            if (payload.clientRequestId && opt.clientRequestId !== payload.clientRequestId) return prev;
            const data = qc.getQueryData<{ messages?: Array<{ id?: string; content?: string }> }>(["studio-tab", projectId, payload.tabId]);
            const found = (data?.messages ?? []).some((m) =>
              m.id === payload.messageId || (typeof m.content === "string" && opt.text.trim() !== "" && m.content.includes(opt.text.slice(0, 24))),
            );
            if (!found) return prev;
            const next = { ...prev };
            delete next[payload.tabId];
            return next;
          });
        }, 400);
      }),
      listen<HistoryChanged>("studio://history-changed", ({ payload }) => {
        scheduleHistory(payload.tabId);
      }),
      listen<DocProgress>("studio://document-progress", ({ payload }) => {
        if (payload.tabId !== activeTabId && payload.tabId !== sendTargetId) return;
        setDocProgress(payload);
      }),
    ]).then((unlisten) => {
      if (disposed) unlisten.forEach((fn) => fn());
      else cleanups.push(...unlisten);
    });
    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
      if (historyTimer.current) clearTimeout(historyTimer.current);
    };
  }, [activeTabId, projectId, sendTargetId, qc]);
  const resolveTool = useMutation({
    mutationFn: (approved: boolean) =>
      studioApi.resolveTool(projectId, activeTabId, approved, modelProfileId ?? undefined),
    onMutate: () => { setProvisional(""); setRunningTool(""); },
    onSuccess: async () => {
      setProvisional("");
      setRunningTool("");
      await Promise.all([refreshTabs(), refreshArtifacts()]);
    },
    onError: (e) => toast.push({ tone: "error", message: aiErrorText(e) }),
  });
  const streaming = send.isPending || resolveTool.isPending;
  const importFiles = useMutation({
    mutationFn: (paths: string[]) =>
      studioApi.importFiles(projectId, activeTabId, paths),
    onSuccess: async (created) => {
      await Promise.all([
        refreshArtifacts(),
        qc.invalidateQueries({ queryKey: ["sources", projectId] }),
      ]);
      toast.push({
        tone: "success",
        message: t("studio.filesImported", { count: created.length }),
      });
    },
    onError: () =>
      toast.push({ tone: "error", message: t("studio.importFailed") }),
  });

  // Tauri's native file-drop event carries real filesystem paths on macOS;
  // HTML drag data in WKWebView does not do so reliably. The listener exists
  // only while Studio is mounted and a conversation can own the artifacts.
  useEffect(() => {
    if (!inTauri || !activeTabId) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/webviewWindow").then(async ({ getCurrentWebviewWindow }) => {
      const stop = await getCurrentWebviewWindow().onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setDropActive(true);
        } else if (event.payload.type === "leave") {
          setDropActive(false);
        } else if (event.payload.type === "drop") {
          setDropActive(false);
          const paths = event.payload.paths.filter(Boolean);
          if (paths.length) importFiles.mutate(paths);
        }
      });
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
      setDropActive(false);
    };
    // The mutation object is stable for the active render; re-registering on
    // each mutation state change would briefly lose native drop events.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTabId, projectId]);

  const importArtifact = useMutation({
    mutationFn: (id: string) => studioApi.importArtifact(projectId, id),
    onSuccess: async (source) => {
      await Promise.all([
        refreshArtifacts(),
        qc.invalidateQueries({ queryKey: ["sources", projectId] }),
      ]);
      onSourceImported(source);
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

  // Follow the conversation as it grows, but never yank the reader away
  // from history they are reviewing (v1.5.0 A-2).
  useEffect(() => {
    const node = messagesRef.current;
    if (!node) return;
    const nearBottom = node.scrollHeight - node.scrollTop - node.clientHeight < 140;
    if (nearBottom) node.scrollTop = node.scrollHeight;
  }, [messages.length, send.isPending, provisional]);

  function submit() {
    if (!text.trim() || !active || locked || send.isPending) return;
    send.mutate({
      payload: text.trim(),
      replaceFrom: editingTurn?.id,
      targetTabId: activeTabId,
      scopeSnapshot: scope,
      profileSnapshot: modelProfileId ?? undefined,
      clientRequestId: newClientRequestId(),
    });
  }

  function editFrom(message: ChatMessage) {
    setEditingTurn({ id: message.id, content: message.content });
    setText(message.content);
    requestAnimationFrame(() => {
      composerRef.current?.focus();
      composerRef.current?.setSelectionRange(message.content.length, message.content.length);
    });
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
      {dropActive ? (
        <div className={styles.dropOverlay} role="status">
          <strong>{t("studio.dropFiles")}</strong>
          <span>{t("studio.dropFilesHint")}</span>
        </div>
      ) : null}
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
                      {t("studio.messageCount", { count: tab.messageCount })}
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

      {!activeRow ? (
        <main className={styles.conversation}>
          <div className={styles.emptyPane}>
            <EmptyState
              title={t("studio.pickConversation")}
              body={t("studio.pickConversationBody")}
              actions={
                <Button
                  variant="primary"
                  loading={create.isPending}
                  onClick={() => create.mutate()}
                >
                  {t("studio.newTab")}
                </Button>
              }
            />
          </div>
        </main>
      ) : (
      <main className={styles.conversation}>
        <div ref={messagesRef} className={styles.messages} aria-live="polite">
          {messages.map((m) => (
            <MessageRow
              key={m.id}
              message={m}
              onCitation={onCitation}
              onAllow={() => resolveTool.mutate(true)}
              onDeny={() => resolveTool.mutate(false)}
              onContinue={() => send.mutate({
                payload: "",
                targetTabId: activeTabId,
                scopeSnapshot: scope,
                profileSnapshot: modelProfileId ?? undefined,
                clientRequestId: newClientRequestId(),
              })}
              onEdit={() => editFrom(m)}
              busy={resolveTool.isPending || send.isPending}
            />
          ))}
          {activeTabId && optimisticByTab[activeTabId] ? (
            <article className={styles.message} data-role="user" data-optimistic="true">
              <div className={styles.messageHead}>
                <span className={styles.role}>{t("studio.role.user")}</span>
                <span className={styles.toolMeta}>{t("studio.sending")}</span>
              </div>
              <Markdown>{optimisticByTab[activeTabId].text}</Markdown>
            </article>
          ) : null}
          {streaming && provisional ? (
            <div className={styles.streaming} aria-label={t("studio.streaming")}><Markdown>{provisional}</Markdown></div>
          ) : null}
          {streaming ? <p className={styles.thinking}>{runningTool ? t("studio.runningTool", { name: runningTool }) : t("studio.thinking")}</p> : null}
          {sendTargetId && send.isPending && sendTargetId !== activeTabId ? (
            <p className={styles.thinking} role="status">{t("studio.workingTab")}</p>
          ) : null}
          {docProgress && (docProgress.tabId === activeTabId) ? (
            <p className={styles.thinking} role="status">
              {t("studio.translateProgress", { done: docProgress.done, total: docProgress.total })}
            </p>
          ) : null}
        </div>
        <p className={styles.liveRegion} aria-live="polite" role="status">{liveMsg}</p>

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
            {editingTurn ? (
              <div className={styles.editingBanner} role="status">
                <span>{t("studio.editingFromHere")}</span>
                <Button
                  size="sm"
                  variant="quiet"
                  onClick={() => {
                    setEditingTurn(null);
                    setText("");
                  }}
                >
                  {t("common.cancel")}
                </Button>
              </div>
            ) : null}
            <Textarea
              ref={composerRef}
              rows={2}
              aria-label={t("studio.message")}
              placeholder={t("studio.messagePlaceholder")}
              value={text}
              disabled={locked}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (shouldSubmitStudioKey(e.key, e.shiftKey, e.nativeEvent.isComposing)) {
                  e.preventDefault();
                  submit();
                }
              }}
            />
            {send.isPending ? (
              <Button variant="quiet" size="sm" onClick={() => studioApi.cancel(sendTargetId || activeTabId)}>
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
            <div className={styles.selectGroup}>
              <select
                className={styles.scopeSel}
                aria-label={t("studio.model")}
                title={t("studio.model")}
                value={modelProfileId ?? ""}
                onChange={(e) => setModelProfileId(e.target.value || null)}
              >
                <option value="">
                  {chatBinding
                    ? t("studio.defaultModel", { name: chatBinding.model })
                    : t("studio.noDefaultModel")}
                </option>
                {modelOptions
                  .filter((p) => p.id !== chatBinding?.profileId)
                  .map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name} · {p.defaultModel}
                    </option>
                  ))}
              </select>
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
            </div>
            <div className={styles.composerActions}>
              {!editingTurn && messages.some((message) => message.role === "user") ? (
                <Button
                  size="sm"
                  variant="quiet"
                  disabled={locked || streaming}
                  onClick={() => {
                    const previous = [...messages].reverse().find((message) => message.role === "user");
                    if (previous) editFrom(previous);
                  }}
                >
                  {t("studio.editPrevious")}
                </Button>
              ) : null}
              <span className={styles.hint}>{t("studio.sendHint")}</span>
            </div>
          </div>
        </form>
      </main>
      )}

      <aside className={styles.workspace} aria-label={t("studio.workspace")}>
        <div className={styles.wsHead}>
          <span className={styles.wsTitle}>{t("studio.workspace")}</span>
          <code className={styles.wsPath}>{t("studio.workspacePath", { project: projectName })}</code>
        </div>
        <p className={styles.dropHint}>{t("studio.dropFilesHint")}</p>
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
  onEdit,
  busy,
}: {
  message: ChatMessage;
  onCitation: (c: Citation) => void;
  onAllow: () => void;
  onDeny: () => void;
  onContinue: () => void;
  onEdit: () => void;
  busy: boolean;
}) {
  const { t } = useTranslation();
  const roleKey = `studio.role.${message.role}`;
  const role = t(roleKey) === roleKey ? message.role : t(roleKey);
  const calls = (message.toolCalls ?? []) as StudioToolCall[];

  if (message.role === "tool") {
    // Collapsed by default (like Claude): the reader opens it only when they
    // want to inspect the raw output.
    const lines = message.content.split("\n").length;
    return (
      <article className={styles.message} data-role="tool">
        <details className={styles.toolBlock}>
          <summary className={styles.toolSummary}>
            <span className={styles.role}>{t("studio.toolResult")}</span>
            <span className={styles.toolMeta}>
              {t("studio.toolResultLines", { count: lines })}
            </span>
          </summary>
          <pre className={styles.toolOut}>{message.content}</pre>
        </details>
      </article>
    );
  }

  return (
    <article className={styles.message} data-role={message.role}>
      <div className={styles.messageHead}>
        <span className={styles.role}>{role}</span>
        {message.role === "user" && message.status === "complete" ? (
          <button type="button" className={styles.editTurn} disabled={busy} onClick={onEdit}>
            {t("studio.editFromHere")}
          </button>
        ) : null}
      </div>
      {message.content ? <Markdown>{message.content}</Markdown> : null}

      {calls.length && message.status !== "pending_approval" ? (
        <details className={styles.toolBlock}>
          <summary className={styles.toolSummary}>
            <span className={styles.role}>{t("studio.activity")}</span>
            <span className={styles.toolMeta}>
              {t("studio.activityCount", { count: calls.length })}
            </span>
          </summary>
          <div className={styles.activityList}>
            {calls.map((call, index) => (
              <section key={call.id ?? index} className={styles.activityItem}>
                <strong>{call.name}</strong>
                <pre className={styles.toolOut}>
                  {typeof call.arguments === "string"
                    ? call.arguments
                    : JSON.stringify(call.arguments, null, 2)}
                </pre>
              </section>
            ))}
          </div>
        </details>
      ) : null}

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
