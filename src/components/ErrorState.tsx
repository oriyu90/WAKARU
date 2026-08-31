import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon } from "../app/Icons";
import { Button } from "./Button";
import { IpcError } from "../ipc/client";
import styles from "./feedback.module.css";

/** What happened (plain language) + a retry + collapsible technical detail
 * (docs/06 §9.2). */
export function ErrorState({
  error,
  onRetry,
}: {
  error: unknown;
  onRetry?: () => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const headline =
    error instanceof IpcError
      ? safeT(t, error.i18nKey)
      : t("errors.internal");
  const detail =
    error instanceof Error ? `${error.name}: ${error.message}` : String(error);

  return (
    <div className={styles.errorState} role="alert">
      <div className={styles.errorHead}>
        <AlertIcon size={16} />
        {headline}
      </div>
      {onRetry ? (
        <Button variant="secondary" size="sm" onClick={onRetry}>
          {t("common.retry")}
        </Button>
      ) : null}
      <button
        type="button"
        className={styles.detailToggle}
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {t("common.details")}
      </button>
      {open ? <pre className={styles.errorDetail}>{detail}</pre> : null}
    </div>
  );
}

function safeT(t: (k: string) => string, key: string) {
  const v = t(key);
  return v === key ? t("errors.internal") : v;
}
