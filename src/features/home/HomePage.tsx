import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { ErrorState } from "../../components/ErrorState";
import { Skeleton } from "../../components/Skeleton";
import { PlusIcon } from "../../app/Icons";
import { projectsApi } from "../../ipc/projects";
import { inTauri } from "../../ipc/client";
import { ProjectCard } from "./ProjectCard";
import { NewProjectDialog } from "./NewProjectDialog";
import styles from "./HomePage.module.css";

export function HomePage() {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState(false);

  const projects = useQuery({
    queryKey: ["projects", false],
    queryFn: () => projectsApi.list(false),
    enabled: inTauri,
  });

  const list = projects.data ?? [];

  return (
    <section className={styles.page}>
      <header className={styles.head}>
        <h1 className={styles.title}>{t("home.title")}</h1>
        <Button
          variant="primary"
          icon={<PlusIcon size={16} />}
          onClick={() => setDialog(true)}
        >
          {t("home.newProject")}
        </Button>
      </header>

      {!inTauri ? (
        <EmptyState
          title={t("errors.backendUnavailable")}
          body={t("home.devHint")}
        />
      ) : projects.isLoading ? (
        <div className={styles.grid} aria-hidden="true">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className={styles.skelCard}>
              <Skeleton width="55%" height="1.1rem" />
              <Skeleton width="80%" height="0.8rem" />
              <Skeleton width="40%" height="0.7rem" />
            </div>
          ))}
        </div>
      ) : projects.isError ? (
        <ErrorState error={projects.error} onRetry={() => projects.refetch()} />
      ) : list.length === 0 ? (
        <EmptyState
          title={t("home.empty.title")}
          body={t("home.empty.body")}
          actions={
            <Button variant="primary" onClick={() => setDialog(true)}>
              {t("home.empty.cta")}
            </Button>
          }
        />
      ) : (
        <div className={styles.grid}>
          {list.map((p) => (
            <ProjectCard key={p.id} project={p} />
          ))}
          <button
            type="button"
            className={styles.addCard}
            onClick={() => setDialog(true)}
          >
            <PlusIcon size={20} />
            {t("home.newProject")}
          </button>
        </div>
      )}

      <NewProjectDialog open={dialog} onClose={() => setDialog(false)} />
    </section>
  );
}
