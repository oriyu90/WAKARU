import type { ReactNode } from "react";
import styles from "./feedback.module.css";

/** Every list and panel has one of these. No decorative illustration — a line of
 * text saying what goes here, and the first action (docs/06 §9.2). */
export function EmptyState({
  title,
  body,
  actions,
}: {
  title: string;
  body?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className={styles.empty}>
      <p className={styles.emptyTitle}>{title}</p>
      {body ? <p className={styles.emptyBody}>{body}</p> : null}
      {actions ? <div className={styles.emptyActions}>{actions}</div> : null}
    </div>
  );
}
