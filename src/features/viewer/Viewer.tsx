import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { HomeIcon, CloseIcon, FileIcon } from "../../app/Icons";
import { Drawer } from "../../components/Drawer";
import { useUiStore } from "../../stores/ui";
import { viewerApi } from "../../ipc/viewer";
import { aiApi } from "../../ipc/ai";
import { inTauri } from "../../ipc/client";
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

export function Viewer({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [active, setActive] = useState<string>(HOME);
  const illustratorEnabled = useUiStore((s) => s.illustratorEnabled);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [ctx, setCtx] = useState<PreviewContext | null>(null);
  const stripRef = useRef<HTMLDivElement>(null);

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

  // Keep the active tab scrolled into view.
  useEffect(() => {
    stripRef.current
      ?.querySelector<HTMLElement>('[data-active="true"]')
      ?.scrollIntoView({ inline: "nearest", block: "nearest" });
  }, [active, tabs.data]);

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

  return (
    <div className={styles.viewer}>
      <div className={styles.strip} ref={stripRef} role="tablist" aria-label={t("project.pane.viewer")}>
        <button
          type="button"
          role="tab"
          aria-selected={active === HOME}
          data-active={active === HOME}
          className={`${styles.tab} ${styles.homeTab}`}
          onClick={() => setActive(HOME)}
          aria-label={t("viewer.sourceList")}
        >
          <HomeIcon size={15} />
        </button>
        {list.map((tb) => (
          <span
            key={tb.id}
            role="tab"
            aria-selected={active === tb.id}
            data-active={active === tb.id}
            className={styles.tab}
            tabIndex={0}
            onClick={() => setActive(tb.id)}
            onAuxClick={(e) => {
              if (e.button === 1) void closeTab(tb.id);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") setActive(tb.id);
            }}
          >
            <FileIcon size={14} />
            <span className={styles.tabName}>{tb.name}</span>
            <button
              type="button"
              className={styles.tabClose}
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

      <div className={styles.stage}>
        <div className={styles.pane}>
          {active === HOME || !activeTab ? (
            <SourceListPanel projectId={projectId} onOpen={openTab} />
          ) : (
            <Preview projectId={projectId} tab={activeTab} onContext={setCtx} />
          )}
        </div>

        {illustratorEnabled ? (
          <>
            <button
              type="button"
              className={styles.handle}
              aria-label={t("viewer.illustratorHandle")}
              aria-expanded={drawerOpen}
              onClick={() => setDrawerOpen((v) => !v)}
            />
            <Drawer
              open={drawerOpen}
              onClose={() => setDrawerOpen(false)}
              label={t("viewer.illustrator")}
            >
              <IllustratorDrawer
                projectId={projectId}
                sourceId={ctx?.sourceId ?? activeTab?.sourceId ?? null}
                locator={ctx?.locator ?? activeTab?.locator ?? { t: "whole" }}
                position={ctx?.position}
                visionSupported={visionSupported}
              />
            </Drawer>
          </>
        ) : null}
      </div>
    </div>
  );
}
