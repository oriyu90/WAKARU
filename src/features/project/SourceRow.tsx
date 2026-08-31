import { useTranslation } from "react-i18next";
import type { Source } from "../../ipc/types.gen";
import { IconButton } from "../../components/IconButton";
import { CloseIcon } from "../../app/Icons";
import { StatusGlyph } from "./statusGlyph";
import styles from "./SourceList.module.css";

function formatBytes(n: number, lang: string) {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${new Intl.NumberFormat(lang, { maximumFractionDigits: 1 }).format(v)} ${units[i]}`;
}

export function SourceRow({
  source,
  onOpen,
  onReanalyze,
  onDelete,
}: {
  source: Source;
  onOpen?: () => void;
  onReanalyze: () => void;
  onDelete: () => void;
}) {
  const { t, i18n } = useTranslation();
  const failed = source.status === "failed";
  const openable = !failed && !!onOpen;

  return (
    <div className={styles.row} data-failed={failed}>
      <span className={styles.rowGlyph}>
        <StatusGlyph status={source.status} />
      </span>
      {openable ? (
        <button
          type="button"
          className={`${styles.rowName} ${styles.rowNameButton}`}
          title={source.originalName}
          onClick={onOpen}
        >
          {source.originalName}
        </button>
      ) : (
        <span className={styles.rowName} title={source.originalName}>
          {source.originalName}
        </span>
      )}
      <span className={styles.rowKind}>{t(`sourceKind.${source.kind}`)}</span>
      <span className={`${styles.rowBytes} u-mono-nums`}>
        {formatBytes(source.bytes, i18n.language)}
      </span>
      {failed ? (
        <span className={styles.rowError}>
          {source.errorCode === "SOURCE_UNSUPPORTED_FORMAT"
            ? t("project.sources.unsupported")
            : (source.errorMessage ?? t("states.failed"))}
        </span>
      ) : null}
      <span className={styles.rowActions}>
        <IconButton
          label={t("project.sources.reanalyze")}
          size="sm"
          onClick={onReanalyze}
          disabled={source.status === "analyzing" || source.status === "queued"}
        >
          <span aria-hidden="true" className={styles.reicon}>↻</span>
        </IconButton>
        <IconButton label={t("project.sources.delete")} size="sm" onClick={onDelete}>
          <CloseIcon size={14} />
        </IconButton>
      </span>
    </div>
  );
}
