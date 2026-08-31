import type { SourceStatus } from "../../ipc/types.gen";
import { Spinner } from "../../components/Spinner";
import styles from "./SourceList.module.css";

/** Status is shown by shape, never colour alone (docs/07 §3). `ready` shows
 * nothing — a badge on every row is noise (docs/06 §5.1). */
export function StatusGlyph({ status }: { status: SourceStatus }) {
  switch (status) {
    case "queued":
      return <span className={styles.glyphQueued} aria-hidden="true" />;
    case "analyzing":
      return <Spinner />;
    case "ready_partial":
      return <span className={styles.glyph} data-shape="triangle" aria-hidden="true">△</span>;
    case "failed":
      return <span className={styles.glyph} data-shape="bang" aria-hidden="true">!</span>;
    case "ready":
    default:
      return null;
  }
}
