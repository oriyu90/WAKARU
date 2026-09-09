import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import styles from "./ContextMenu.module.css";

export type ContextMenuItem = {
  label: string;
  onSelect: () => void;
  danger?: boolean;
};

/** A pointer-summoned menu (right-click / the context-menu key). Portalled to
 * <body> so the summoning element's `overflow` can't clip it. Closes on Escape,
 * outside pointer-down, scroll, resize and blur; focus returns to `returnFocus`.
 * `role="menu"` with ↑/↓ roving focus and Enter/Space activation. */
export function ContextMenu({
  x,
  y,
  items,
  label,
  onClose,
  returnFocus,
}: {
  x: number;
  y: number;
  items: ContextMenuItem[];
  label: string;
  onClose: () => void;
  returnFocus?: HTMLElement | null;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  // Clamp inside the viewport once we know the menu's size.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const pad = 8;
    setPos({
      left: Math.max(pad, Math.min(x, window.innerWidth - r.width - pad)),
      top: Math.max(pad, Math.min(y, window.innerHeight - r.height - pad)),
    });
    el.querySelector<HTMLButtonElement>("[role='menuitem']")?.focus();
  }, [x, y]);

  useEffect(() => {
    const close = () => onClose();
    const onDown = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      const el = ref.current;
      if (!el) return;
      const menuItems = Array.from(
        el.querySelectorAll<HTMLButtonElement>("[role='menuitem']"),
      );
      const i = menuItems.indexOf(document.activeElement as HTMLButtonElement);
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        menuItems[(i + 1) % menuItems.length]?.focus();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        menuItems[(i - 1 + menuItems.length) % menuItems.length]?.focus();
      } else if (e.key === "Home") {
        e.preventDefault();
        menuItems[0]?.focus();
      } else if (e.key === "End") {
        e.preventDefault();
        menuItems[menuItems.length - 1]?.focus();
      } else if (e.key === "Tab") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("blur", close);
      returnFocus?.focus?.();
    };
  }, [onClose, returnFocus]);

  return createPortal(
    <div
      ref={ref}
      className={styles.menu}
      role="menu"
      aria-label={label}
      style={{ left: pos.left, top: pos.top }}
    >
      {items.map((item, idx) => (
        <button
          key={idx}
          type="button"
          role="menuitem"
          tabIndex={-1}
          className={item.danger ? styles.itemDanger : styles.item}
          onClick={() => {
            onClose();
            item.onSelect();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>,
    document.getElementById("app") ?? document.body,
  );
}
