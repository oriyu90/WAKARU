import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { IconButton } from "./IconButton";
import { CloseIcon } from "../app/Icons";
import styles from "./Dialog.module.css";

/** Native <dialog>: focus trap, Esc-to-close and ::backdrop come for free. Pinned
 * with margin:auto so custom positioning can't snap it to a corner
 * (docs/06 §9 / Hallmark interaction-and-states). */
export function Dialog({
  open,
  onClose,
  title,
  children,
  footer,
  width,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  width?: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) el.showModal();
    else if (!open && el.open) el.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      className={styles.dialog}
      style={width ? { width } : undefined}
      onClose={onClose}
      onCancel={onClose}
      onClick={(e) => {
        if (e.target === ref.current) onClose(); // backdrop click
      }}
      aria-labelledby="dialog-title"
    >
      <div className={styles.inner}>
        <header className={styles.head}>
          <h2 id="dialog-title" className={styles.title}>
            {title}
          </h2>
          <IconButton label="Close" onClick={onClose}>
            <CloseIcon />
          </IconButton>
        </header>
        <div className={styles.body}>{children}</div>
        {footer ? <footer className={styles.foot}>{footer}</footer> : null}
      </div>
    </dialog>
  );
}
