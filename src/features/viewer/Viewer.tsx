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
import type { ViewerTab } from "../../ipc/types.gen";
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

/** A request from elsewhere (e.g. a Studio citation) to open a source at a
 * locator. `nonce` changes each time so repeat clicks on the same source
 * re-fire the effect. */
export type ViewerFocusRequest = {
  sourceId: string;
  locator?: unknown;
  nonce: number;
};

export function Viewer({
  projectId,
  focusRequest,
}: {
  projectId: string;
  focusRequest?: ViewerFocusRequest | null;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const toast = useToast();
  const [active, setActive] = useState<string>(HOME);
  const illustratorEnabled = useUiStore((s) => s.illustratorEnabled);
  const setIllustratorEnabled = useUiStore((s) => s.setIllustratorEnabled);
  const { patch } = useAppSettings();
  const [drawerOpen, setDrawerOpen] = useState(false);
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
          >
            <IllustratorDrawer
              projectId={projectId}
              sourceId={ctx?.sourceId ?? activeTab?.sourceId ?? null}
              visionSupported={visionSupported}
              autoRun={!justEnabled}
              onStarted={() => setJustEnabled(false)}
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
