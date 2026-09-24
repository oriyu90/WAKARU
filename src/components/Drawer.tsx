import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { trapTab } from "./focus";
import styles from "./Drawer.module.css";

function rootFontPx(): number {
  const v = parseFloat(getComputedStyle(document.documentElement).fontSize);
  return Number.isFinite(v) && v > 0 ? v : 16;
}

/** A side panel that sits *beside* content (it does not overlay — the sidebar
 * does that). Used for the Live Illustrator drawer (P4). `side="right"`.
 *
 * With `onWidthChange` the leading edge becomes a drag handle (keyboard
 * accessible). Widths are `rem` so display scaling keeps working; the caller
 * owns persistence. `max-inline-size: 100%` in CSS caps the panel to its
 * container, so growing the drawer narrows the document pane instead of
 * overflowing. */
export function Drawer({
  open,
  onClose,
  side = "right",
  label,
  width,
  widthRem,
  minWidthRem = 24,
  maxWidthRem = 44,
  onWidthChange,
  resizeLabel,
  children,
}: {
  open: boolean;
  onClose: () => void;
  side?: "right" | "left";
  label: string;
  width?: string;
  widthRem?: number;
  minWidthRem?: number;
  maxWidthRem?: number;
  onWidthChange?: (rem: number) => void;
  resizeLabel?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const drag = useRef<{ startX: number; startW: number } | null>(null);
  const [resizing, setResizing] = useState(false);

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

  const resizable = open && !!onWidthChange && widthRem != null;
  const clampRem = (r: number) =>
    Math.min(maxWidthRem, Math.max(minWidthRem, r));

  function endDrag() {
    drag.current = null;
    setResizing(false);
  }

  return (
    <div
      ref={ref}
      className={styles.drawer}
      data-open={open}
      data-side={side}
      data-resizing={resizing || undefined}
      style={
        widthRem != null
          ? { inlineSize: `${clampRem(widthRem)}rem` }
          : width
            ? { inlineSize: width }
            : undefined
      }
      role="region"
      aria-label={label}
      aria-hidden={!open}
    >
      {resizable ? (
        <div
          className={styles.handle}
          data-side={side}
          role="separator"
          aria-orientation="vertical"
          aria-label={resizeLabel ?? "Resize panel"}
          aria-valuemin={minWidthRem}
          aria-valuemax={maxWidthRem}
          aria-valuenow={Math.round(clampRem(widthRem) * 10) / 10}
          tabIndex={0}
          onPointerDown={(e) => {
            if (e.button !== 0) return;
            e.preventDefault();
            e.currentTarget.setPointerCapture?.(e.pointerId);
            drag.current = {
              startX: e.clientX,
              startW:
                ref.current?.getBoundingClientRect().width ??
                minWidthRem * rootFontPx(),
            };
            setResizing(true);
          }}
          onPointerMove={(e) => {
            const d = drag.current;
            if (!d) return;
            const dx =
              side === "right" ? d.startX - e.clientX : e.clientX - d.startX;
            onWidthChange?.(clampRem((d.startW + dx) / rootFontPx()));
          }}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onKeyDown={(e) => {
            const step =
              e.key === "ArrowLeft"
                ? -1
                : e.key === "ArrowRight"
                  ? 1
                  : null;
            if (step !== null) {
              e.preventDefault();
              onWidthChange?.(clampRem(widthRem + step));
            } else if (e.key === "Home") {
              e.preventDefault();
              onWidthChange?.(minWidthRem);
            } else if (e.key === "End") {
              e.preventDefault();
              onWidthChange?.(maxWidthRem);
            }
          }}
        />
      ) : null}
      {open ? children : null}
    </div>
  );
}
