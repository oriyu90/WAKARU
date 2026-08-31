import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import type { ProjectSummary } from "../../ipc/types.gen";
import styles from "./ProjectCard.module.css";

export function ProjectCard({ project }: { project: ProjectSummary }) {
  const { t } = useTranslation();
  const { i18n } = useTranslation();
  const opened = project.lastOpenedAt
    ? new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium" }).format(
        new Date(project.lastOpenedAt),
      )
    : null;

  return (
    <Link to={`/p/${project.id}`} className={styles.card} data-color={project.color}>
      <span className={styles.rail} aria-hidden="true" />
      <span className={styles.name}>{project.name}</span>
      {project.description ? (
        <span className={styles.desc}>{project.description}</span>
      ) : null}
      <span className={styles.meta}>
        <span className="u-mono-nums">
          {t("home.card.sources", { count: project.sourceCount })}
        </span>
        {opened ? <span className={styles.dot}>·</span> : null}
        {opened ? <span>{opened}</span> : null}
        {project.archived ? (
          <span className={styles.archived}>📦</span>
        ) : null}
      </span>
    </Link>
  );
}
