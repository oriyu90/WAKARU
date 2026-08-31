import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { trapTab } from "./focus";
import styles from "./Drawer.module.css";

/** A side panel that sits *beside* content (it does not overlay — the sidebar
 * does that). Used for the Live Illustrator drawer (P4). `side="right"`. */
export function Drawer({
  open,
  onClose,
  side = "right",
  label,
  width,
  children,
}: {
  open: boolean;
  onClose: () => void;
  side?: "right" | "left";
  label: string;
  width?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const el = ref.current;
    if (!el) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      else trapTab(el, e);
    };
    el.addEventListener("keydown", onKey);
    return () => el.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  return (
    <div
      ref={ref}
      className={styles.drawer}
      data-open={open}
      data-side={side}
      style={width ? { inlineSize: width } : undefined}
      role="region"
      aria-label={label}
      aria-hidden={!open}
    >
      {open ? children : null}
    </div>
  );
}
