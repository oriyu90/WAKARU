import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { HomeIcon, CloseIcon, FileIcon, PlusIcon, LinkIcon, PanelRightIcon } from "../../app/Icons";
import { Drawer } from "../../components/Drawer";
import { Button } from "../../components/Button";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Input } from "../../components/Input";
import { useToast } from "../../components/useToast";
import { useUiStore } from "../../stores/ui";
import { useAppSettings } from "../settings/useAppSettings";
import { viewerApi } from "../../ipc/viewer";
import { aiApi } from "../../ipc/ai";
import { sourcesApi, pickSourceFiles, pickFolder } from "../../ipc/sources";
import { IpcError, inTauri } from "../../ipc/client";
import type { Citation, SourceStatusEvent, ViewerTab } from "../../ipc/types.gen";
import { SourceListPanel } from "../project/SourceListPanel";
import { Preview } from "./Preview";
import { IllustratorDrawer } from "./IllustratorDrawer";
import styles from "./Viewer.module.css";

export type PreviewContext = {
  sourceId: string;
  locator: unknown;
  position?: string;
};

const HOME = "__home__";
const ACTIVE_TAB_KEY = "wakaru.viewer.activeTab";

function loadActiveTab(projectId: string): string {
  try {
    return localStorage.getItem(`${ACTIVE_TAB_KEY}.${projectId}`) ?? HOME;
  } catch {
    return HOME;
  }
}

function saveActiveTab(projectId: string, id: string) {
  try {
    localStorage.setItem(`${ACTIVE_TAB_KEY}.${projectId}`, id);
  } catch {
    // Private mode etc. — the tab still stays open for this session.
  }
}

/** A request from elsewhere (e.g. a Studio citation) to open a source at a
 * locator. `nonce` changes each time so repeat clicks on the same source
 * re-fire the effect. */
export type ViewerFocusRequest = {
  sourceId: string;
  locator?: unknown;
  nonce: number;
};

const ILL_WIDTH_KEY = "wakaru.illustrator.widthRem";
const ILL_WIDTH_MIN = 24;
const ILL_WIDTH_MAX = 44;
const ILL_WIDTH_DEFAULT = 24;

function loadIllustratorWidth(): number {
  try {
    const raw = localStorage.getItem(ILL_WIDTH_KEY);
    const v = raw == null ? NaN : Number.parseFloat(raw);
    if (!Number.isFinite(v)) return ILL_WIDTH_DEFAULT;
    return Math.min(ILL_WIDTH_MAX, Math.max(ILL_WIDTH_MIN, v));
  } catch {
    return ILL_WIDTH_DEFAULT;
  }
}

export function Viewer({
  projectId,
  focusRequest,
  onCitation,
  onPreviewingChange,
}: {
  projectId: string;
  focusRequest?: ViewerFocusRequest | null;
  onCitation?: (c: Citation) => void;
  /** True while a document tab (not the source list) is open. */
  onPreviewingChange?: (previewing: boolean) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  // The open document survives unmounts (e.g. a settings round-trip):
  // the active tab id is restored per project, then validated once the
  // tab list loads (a closed tab falls back to the source list).
  const [active, setActive] = useState<string>(() => loadActiveTab(projectId));
  const reconciledProject = useRef<string | null>(null);
  useEffect(() => {
    setActive(loadActiveTab(projectId));
  }, [projectId]);
  useEffect(() => {
    saveActiveTab(projectId, active);
  }, [projectId, active]);
  const illustratorEnabled = useUiStore((s) => s.illustratorEnabled);
  const setIllustratorEnabled = useUiStore((s) => s.setIllustratorEnabled);
  const { patch } = useAppSettings();
  const [drawerOpen, setDrawerOpen] = useState(false);
  // Resizable Live panel: the current design width is the minimum; growing
  // the panel narrows the document pane (flex layout + max-inline-size).
  // Persisted locally so it survives restarts.
  const [illWidthRem, setIllWidthRem] = useState<number>(loadIllustratorWidth);
  useEffect(() => {
    try {
      localStorage.setItem(ILL_WIDTH_KEY, String(illWidthRem));
    } catch {
      // Private mode etc. — the panel still resizes for this session.
    }
  }, [illWidthRem]);
  // True only for the first render after the reader flips Live Illustrator on,
  // so the panel opens but waits for one tap before spending tokens.
  const [justEnabled, setJustEnabled] = useState(false);
  const [ctx, setCtx] = useState<PreviewContext | null>(null);
  const [urlDialog, setUrlDialog] = useState(false);
  const [url, setUrl] = useState("");
  const railRef = useRef<HTMLDivElement>(null);

  const visionRole = useQuery({
    queryKey: ["ai-bindings"],
    queryFn: aiApi.getRoleBindings,
    enabled: inTauri && illustratorEnabled,
  });
  const profiles = useQuery({
    queryKey: ["ai-profiles"],
    queryFn: aiApi.listProfiles,
    enabled: inTauri && illustratorEnabled,
  });
  const chatProfileId = visionRole.data?.chat?.profileId;
  const visionSupported =
    profiles.data?.find((p) => p.id === chatProfileId)?.supportsVision ?? false;

  const tabs = useQuery({
    queryKey: ["viewer-tabs", projectId],
    queryFn: () => viewerApi.getTabs(projectId),
    enabled: inTauri,
  });

  // Studio artifacts can be opened while ingestion is still queued. Refresh
  // every matching preview cache when the source becomes ready so the final
  // Markdown/PDF/Office renderer appears without reopening the project.
  useEffect(() => {
    if (!inTauri) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const stop = await listen<SourceStatusEvent>("source://status", ({ payload }) => {
        if (payload.projectId !== projectId) return;
        void Promise.all([
          qc.invalidateQueries({ queryKey: ["sources", projectId] }),
          qc.invalidateQueries({ queryKey: ["source-detail", projectId, payload.sourceId] }),
          qc.invalidateQueries({ queryKey: ["document", projectId, payload.sourceId] }),
          qc.invalidateQueries({ queryKey: ["asset-buffer", payload.sourceId] }),
          qc.invalidateQueries({ queryKey: ["asset-text", projectId, payload.sourceId] }),
          qc.invalidateQueries({ queryKey: ["reader", payload.sourceId] }),
        ]);
      });
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [projectId, qc]);

  const sourcesKey = ["sources", projectId];
  const addFiles = useMutation({
    mutationFn: (paths: string[]) => sourcesApi.addFiles(projectId, paths),
    onSuccess: () => qc.invalidateQueries({ queryKey: sourcesKey }),
    onError: (err) =>
      toast.push({
        tone: "error",
        message:
          err instanceof IpcError
            ? t([`errors.${err.code}`, "errors.internal"])
            : t("errors.internal"),
      }),
  });
  const addUrl = useMutation({
    mutationFn: (u: string) => sourcesApi.addUrl(projectId, u),
    onSuccess: () => {
      setUrl("");
      setUrlDialog(false);
      qc.invalidateQueries({ queryKey: sourcesKey });
    },
    onError: (err) =>
      toast.push({
        tone: "error",
        message:
          err instanceof IpcError
            ? t([`errors.${err.code}`, "errors.internal"])
            : t("errors.internal"),
      }),
  });

  const addFolder = useMutation({
    mutationFn: (folder: string) => sourcesApi.addFolder(projectId, folder),
    onSuccess: () => qc.invalidateQueries({ queryKey: sourcesKey }),
    onError: (err) =>
      toast.push({
        tone: "error",
        message:
          err instanceof IpcError
            ? t([`errors.${err.code}`, "errors.internal"])
            : t("errors.internal"),
      }),
  });

  async function onAddClick() {
    const paths = await pickSourceFiles();
    if (paths.length) addFiles.mutate(paths);
  }

  async function onAddWebsiteClick() {
    const folder = await pickFolder();
    if (folder) addFolder.mutate(folder);
  }

  const openTab = async (sourceId: string) => {
    const tab = await viewerApi.openTab(projectId, sourceId);
    await qc.invalidateQueries({ queryKey: ["viewer-tabs", projectId] });
    setActive(tab.id);
  };

  const closeTab = async (tabId: string) => {
    await viewerApi.closeTab(projectId, tabId);
    await qc.invalidateQueries({ queryKey: ["viewer-tabs", projectId] });
    setActive((cur) => (cur === tabId ? HOME : cur));
  };

  // Open + focus a source when asked from outside (Studio citation jump).
  useEffect(() => {
    if (!focusRequest || !inTauri) return;
    let cancelled = false;
    void (async () => {
      const tab = await viewerApi.openTab(
        projectId,
        focusRequest.sourceId,
        focusRequest.locator ?? undefined,
      );
      if (cancelled) return;
      await qc.invalidateQueries({ queryKey: ["viewer-tabs", projectId] });
      setActive(tab.id);
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusRequest?.nonce]);

  // Keep the active rail tab scrolled into view.
  useEffect(() => {
    railRef.current
      ?.querySelector<HTMLElement>('[data-active="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [active, tabs.data]);

  // Validate a restored tab id once per project: a tab closed elsewhere
  // falls back to the source list instead of a dead selection.
  useEffect(() => {
    if (!tabs.data || reconciledProject.current === projectId) return;
    reconciledProject.current = projectId;
    setActive((cur) => {
      if (cur === HOME) return cur;
      return tabs.data!.some((tb) => tb.id === cur) ? cur : HOME;
    });
  }, [projectId, tabs.data]);

  // When Live Illustrator is on, the panel comes up automatically as soon as a
  // document is open (issue 6). Closing it stays closed until the next doc.
  useEffect(() => {
    if (illustratorEnabled && active !== HOME) setDrawerOpen(true);
  }, [active, illustratorEnabled]);

  // Cmd/Ctrl+\ toggles the Illustrator drawer (docs/06 §3.2).
  useEffect(() => {
    if (!illustratorEnabled) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "\\") {
        e.preventDefault();
        setDrawerOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [illustratorEnabled]);

  const list: ViewerTab[] = tabs.data ?? [];
  const activeTab = list.find((tb) => tb.id === active);

  // Tell the project pane bar whether a document is on screen so the
  // document adjustment controls can enable themselves.
  useEffect(() => {
    onPreviewingChange?.(active !== HOME && !!activeTab);
  }, [active, activeTab, onPreviewingChange]);

  function toggleIllustrator() {
    if (!illustratorEnabled) {
      setIllustratorEnabled(true);
      patch({ illustrator: { enabled: true } });
      setJustEnabled(true);
      setDrawerOpen(true);
    } else {
      setDrawerOpen((v) => !v);
    }
  }

  return (
    <div className={styles.viewer}>
      <div
        ref={railRef}
        className={styles.rail}
        role="tablist"
        aria-orientation="vertical"
        aria-label={t("project.pane.viewer")}
      >
        <div className={styles.railScroll}>
          <button
            type="button"
            role="tab"
            aria-selected={active === HOME}
            data-active={active === HOME}
            className={`${styles.railTab} ${styles.railHome}`}
            onClick={() => setActive(HOME)}
          >
            <HomeIcon size={16} />
            <span className={styles.railName}>{t("viewer.sourceList")}</span>
          </button>

          {list.map((tb) => (
            <span
              key={tb.id}
              role="tab"
              aria-selected={active === tb.id}
              data-active={active === tb.id}
              className={styles.railTab}
              tabIndex={0}
              title={tb.name}
              onClick={() => setActive(tb.id)}
              onAuxClick={(e) => {
                if (e.button === 1) void closeTab(tb.id);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") setActive(tb.id);
              }}
            >
              <FileIcon size={14} />
              <span className={styles.railName}>{tb.name}</span>
              <button
                type="button"
                className={styles.railClose}
                aria-label={t("common.close")}
                onClick={(e) => {
                  e.stopPropagation();
                  void closeTab(tb.id);
                }}
              >
                <CloseIcon size={12} />
              </button>
            </span>
          ))}
        </div>

        <div className={styles.railFoot}>
          <Button
            size="sm"
            variant="primary"
            block
            icon={<PlusIcon size={14} />}
            loading={addFiles.isPending}
            onClick={onAddClick}
          >
            {t("project.sources.add")}
          </Button>
          <Button
            size="sm"
            variant="quiet"
            block
            icon={<LinkIcon size={14} />}
            onClick={() => setUrlDialog(true)}
          >
            {t("project.sources.addLink")}
          </Button>
          <Button
            size="sm"
            variant="quiet"
            block
            icon={<FileIcon size={14} />}
            loading={addFolder.isPending}
            onClick={onAddWebsiteClick}
          >
            {t("viewer.addWebsite")}
          </Button>
          <button
            type="button"
            className={styles.railToggle}
            data-on={illustratorEnabled}
            aria-pressed={illustratorEnabled ? drawerOpen : false}
            aria-label={
              illustratorEnabled
                ? drawerOpen
                  ? t("viewer.illustratorHide")
                  : t("viewer.illustratorShow")
                : t("viewer.illustratorEnable")
            }
            onClick={toggleIllustrator}
          >
            <PanelRightIcon size={15} />
            <span className={styles.railName}>{t("viewer.illustrator")}</span>
          </button>
        </div>
      </div>

      <div className={styles.stage}>
        <div className={styles.pane}>
          {active === HOME || !activeTab ? (
            <SourceListPanel projectId={projectId} onOpen={openTab} />
          ) : (
            <Preview projectId={projectId} tab={activeTab} onContext={setCtx} />
          )}
        </div>

        {illustratorEnabled ? (
          <Drawer
            open={drawerOpen}
            onClose={() => setDrawerOpen(false)}
            label={t("viewer.illustrator")}
            widthRem={illWidthRem}
            minWidthRem={ILL_WIDTH_MIN}
            maxWidthRem={ILL_WIDTH_MAX}
            onWidthChange={setIllWidthRem}
            resizeLabel={t("illustrator.resizeHandle")}
          >
            <IllustratorDrawer
              projectId={projectId}
              sourceId={ctx?.sourceId ?? activeTab?.sourceId ?? null}
              locator={ctx?.locator ?? activeTab?.locator ?? { t: "whole" }}
              visionSupported={visionSupported}
              autoRun={!justEnabled}
              onStarted={() => setJustEnabled(false)}
              onCitation={onCitation}
            />
          </Drawer>
        ) : null}
      </div>

      <Dialog
        open={urlDialog}
        onClose={() => setUrlDialog(false)}
        title={t("project.sources.addLink")}
        footer={
          <>
            <Button variant="quiet" onClick={() => setUrlDialog(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              loading={addUrl.isPending}
              disabled={!/^https?:\/\//i.test(url.trim())}
              onClick={() => addUrl.mutate(url.trim())}
            >
              {t("project.sources.add")}
            </Button>
          </>
        }
      >
        <Field label="URL" hint={t("project.sources.linkHint")}>
          {({ id }) => (
            <Input
              id={id}
              type="url"
              value={url}
              autoFocus
              placeholder="https://example.com/article"
              onChange={(e) => setUrl(e.target.value)}
            />
          )}
        </Field>
      </Dialog>
    </div>
  );
}
