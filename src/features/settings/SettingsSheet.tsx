import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { CloseIcon } from "../../app/Icons";
import { IconButton } from "../../components/IconButton";
import { trapTab } from "../../components/focus";
import { SettingsPage } from "./SettingsPage";
import styles from "./SettingsSheet.module.css";

/** Settings as an overlay sheet: opening it never navigates, so the current
 * screen (open document, Studio draft, scroll position) stays exactly as it
 * was. Mirrors the sidebar overlay contract (absolute inside #app, Esc to
 * close, focus trap + restoration). */
export function SettingsSheet({
  onClose,
  returnFocus,
}: {
  onClose: () => void;
  returnFocus?: HTMLElement | null;
}) {
  const { t } = useTranslation();
  const ref = useRef<HTMLElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeRef.current?.focus();
    const el = ref.current;
    if (!el) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      else trapTab(el, e);
    };
    el.addEventListener("keydown", onKey);
    return () => el.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(
    () => () => {
      returnFocus?.focus();
    },
    // Restore focus only on unmount.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  return (
    <>
      <button
        type="button"
        className={styles.scrim}
        aria-hidden
        tabIndex={-1}
        onClick={onClose}
      />
      <aside
        ref={ref}
        className={styles.sheet}
        role="dialog"
        aria-modal="true"
        aria-label={t("settings.title")}
      >
        <header className={styles.head}>
          <span className={styles.title}>{t("settings.title")}</span>
          <IconButton
            ref={closeRef}
            label={t("common.close")}
            onClick={onClose}
          >
            <CloseIcon size={15} />
          </IconButton>
        </header>
        <div className={styles.body}>
          <SettingsPage />
        </div>
      </aside>
    </>
  );
}
