import { useCallback, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AlertIcon, CheckIcon, InfoIcon, CloseIcon } from "../app/Icons";
import { IconButton } from "./IconButton";
import { ToastContext } from "./toast-context";
import type { Toast } from "./toast-context";
import styles from "./Toast.module.css";

const ICONS = {
  error: <AlertIcon size={16} />,
  success: <CheckIcon size={16} />,
  info: <InfoIcon size={16} />,
};

const AUTO_DISMISS_MS = 7000;

/** Toasts are for failures, off-screen effects and explicit confirmations only
 * (design.md). They stack at a corner and never shift layout. */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  const dismiss = useCallback((id: string) => {
    setToasts((list) => list.filter((t) => t.id !== id));
    const timer = timers.current[id];
    if (timer) {
      clearTimeout(timer);
      delete timers.current[id];
    }
  }, []);

  const push = useCallback(
    (t: Omit<Toast, "id">) => {
      const id = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
      setToasts((list) => [...list, { ...t, id }]);
      timers.current[id] = setTimeout(() => dismiss(id), AUTO_DISMISS_MS);
      return id;
    },
    [dismiss],
  );

  const api = useMemo(() => ({ push, dismiss }), [push, dismiss]);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className={styles.region} role="region" aria-label="Notifications">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={styles.toast}
            data-tone={t.tone}
            role={t.tone === "error" ? "alert" : "status"}
          >
            <span className={styles.icon} aria-hidden="true">
              {ICONS[t.tone]}
            </span>
            <span className={styles.message}>{t.message}</span>
            {t.action ? (
              <button
                type="button"
                className={styles.action}
                onClick={() => {
                  t.action?.onClick();
                  dismiss(t.id);
                }}
              >
                {t.action.label}
              </button>
            ) : null}
            <IconButton label="Dismiss" size="sm" onClick={() => dismiss(t.id)}>
              <CloseIcon size={14} />
            </IconButton>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
