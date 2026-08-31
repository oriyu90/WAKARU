import type { CSSProperties } from "react";
import styles from "./feedback.module.css";

/** Skeletons mirror the real layout's shape (Hallmark: skeleton over spinner
 * whenever the layout is known). */
export function Skeleton({
  width = "100%",
  height = "1rem",
  radius,
}: {
  width?: string | number;
  height?: string | number;
  radius?: string;
}) {
  const style: CSSProperties = { width, height };
  if (radius) style.borderRadius = radius;
  return <span className={styles.skeleton} style={style} aria-hidden="true" />;
}
