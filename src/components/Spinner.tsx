import styles from "./feedback.module.css";

export function Spinner({ label }: { label?: string }) {
  return (
    <span
      className={styles.spinner}
      role="status"
      aria-live="polite"
      aria-label={label}
    />
  );
}
