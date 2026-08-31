import { useParams } from "react-router";
import styles from "../_stub.module.css";

export function ProjectPage() {
  const { projectId } = useParams();
  return (
    <section className={styles.page}>
      <h1 className={styles.title}>Project</h1>
      <p className={styles.subtitle}>
        Viewer and Studio panes arrive in Phase 2 and Phase 6.
      </p>
      <p className={styles.note}>id: {projectId}</p>
    </section>
  );
}
