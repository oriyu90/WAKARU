import { useEffect, useState } from "react";
import { useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Tabs } from "../../components/Tabs";
import { Button } from "../../components/Button";
import { ErrorState } from "../../components/ErrorState";
import { useUiStore } from "../../stores/ui";
import { projectsApi } from "../../ipc/projects";
import { sourcesApi } from "../../ipc/sources";
import { inTauri } from "../../ipc/client";
import type { Citation, Source } from "../../ipc/types.gen";
import { Viewer } from "../viewer/Viewer";
import type { ViewerFocusRequest } from "../viewer/Viewer";
import { Studio } from "../studio/Studio";
import styles from "./ProjectPage.module.css";

type Pane = "viewer" | "studio";

/** Document rendering adjustments (PDF/DOCX/PPTX) beside the pane tabs.
 * Enabled only while a document preview is on screen. */
function DocAdjustControls({ disabled }: { disabled: boolean }) {
  const { t } = useTranslation();
  const inverted = useUiStore((s) => s.docInverted);
  const clarity = useUiStore((s) => s.docClarity);
  const setInverted = useUiStore((s) => s.setDocInverted);
  const setClarity = useUiStore((s) => s.setDocClarity);
  return (
    <div className={styles.docAdjust}>
      <Button
        size="sm"
        variant={inverted ? "secondary" : "quiet"}
        aria-pressed={inverted}
        disabled={disabled}
        onClick={() => setInverted(!inverted)}
      >
        {t("viewer.invertDocument")}
      </Button>
      <label className={styles.clarity}>
        <span>{t("viewer.clarity")}</span>
        <input
          type="range"
          min="0"
          max="100"
          step="5"
          value={clarity}
          disabled={disabled}
          aria-label={t("viewer.clarity")}
          onChange={(e) => setClarity(Number(e.target.value))}
        />
        <output className="u-mono-nums">{clarity}</output>
      </label>
    </div>
  );
}

export function ProjectPage() {
  const { projectId } = useParams();
  const { t } = useTranslation();
  const [pane, setPane] = useState<Pane>("viewer");
  const [focus, setFocus] = useState<ViewerFocusRequest | null>(null);
  const [previewing, setPreviewing] = useState(false);

  const project = useQuery({
    queryKey: ["project", projectId],
    queryFn: () => projectsApi.open(projectId!),
    enabled: inTauri && !!projectId,
  });

  const sources = useQuery({
    queryKey: ["sources", projectId],
    queryFn: () => sourcesApi.list(projectId!),
    enabled: inTauri && !!projectId,
  });

  useEffect(() => {
    if (project.data) document.title = `WAKARU · ${project.data.name}`;
    return () => {
      document.title = "WAKARU";
    };
  }, [project.data]);

  if (!projectId) return null;
  if (project.isError) {
    return (
      <div className={styles.page}>
        <ErrorState error={project.error} onRetry={() => project.refetch()} />
      </div>
    );
  }

  const onCitation = (c: Citation) => {
    if (!c.sourceId) return;
    setFocus({ sourceId: c.sourceId, locator: c.locator, nonce: Date.now() });
    setPane("viewer");
  };

  const onSourceImported = (source: Source) => {
    setFocus({ sourceId: source.id, nonce: Date.now() });
    setPane("viewer");
  };

  return (
    <div className={styles.page}>
      <div className={styles.paneBar}>
        <Tabs
          label={t("nav.projects")}
          variant="segmented"
          value={pane}
          onChange={(v) => setPane(v as Pane)}
          items={[
            { id: "viewer", label: t("project.pane.viewer") },
            { id: "studio", label: t("project.pane.studio") },
          ]}
        />
        {pane === "viewer" ? (
          <DocAdjustControls disabled={!previewing} />
        ) : null}
      </div>

      <div className={styles.paneBody}>
        <div hidden={pane !== "viewer"} className={styles.paneFill}>
          <Viewer
            projectId={projectId}
            focusRequest={focus}
            onCitation={onCitation}
            onPreviewingChange={setPreviewing}
          />
        </div>
        <div hidden={pane !== "studio"} className={styles.paneFill}>
          <Studio
            projectId={projectId}
            projectName={project.data?.name ?? ""}
            sources={sources.data ?? []}
            onCitation={onCitation}
            onSourceImported={onSourceImported}
          />
        </div>
      </div>
    </div>
  );
}
