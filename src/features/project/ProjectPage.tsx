import { useEffect, useState } from "react";
import { useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Tabs } from "../../components/Tabs";
import { ErrorState } from "../../components/ErrorState";
import { projectsApi } from "../../ipc/projects";
import { inTauri } from "../../ipc/client";
import { Viewer } from "../viewer/Viewer";
import styles from "./ProjectPage.module.css";

type Pane = "viewer" | "studio";

export function ProjectPage() {
  const { projectId } = useParams();
  const { t } = useTranslation();
  const [pane, setPane] = useState<Pane>("viewer");

  const project = useQuery({
    queryKey: ["project", projectId],
    queryFn: () => projectsApi.open(projectId!),
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
      </div>

      <div className={styles.paneBody}>
        {pane === "viewer" ? (
          <Viewer projectId={projectId} />
        ) : (
          <p className={styles.placeholder}>{t("project.studioPlaceholder")}</p>
        )}
      </div>
    </div>
  );
}
