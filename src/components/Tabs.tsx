import { useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import styles from "./Tabs.module.css";

export type TabItem = {
  id: string;
  label: ReactNode;
  /** Icon-only tabs still need a name for SR. */
  ariaLabel?: string;
  /** A tab that is not currently selectable (e.g. no page in view yet). */
  disabled?: boolean;
};

/** WAI-ARIA tablist: arrow keys move, Home/End jump, Tab order matches the eye
 * (docs/06 §10). `variant="segmented"` is the pane switch; `"underline"` is the
 * file-tab strip. */
export function Tabs({
  items,
  value,
  onChange,
  variant = "underline",
  label,
}: {
  items: TabItem[];
  value: string;
  onChange: (id: string) => void;
  variant?: "underline" | "segmented";
  label: string;
}) {
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});

  function onKeyDown(e: KeyboardEvent) {
    const idx = items.findIndex((t) => t.id === value);
    if (idx < 0) return;
    let next = idx;
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (idx + 1) % items.length;
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp")
      next = (idx - 1 + items.length) % items.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = items.length - 1;
    else return;
    e.preventDefault();
    // Skip over disabled tabs in the direction of travel.
    const forward = next >= idx && !(idx === items.length - 1 && next === 0);
    while (items[next]?.disabled && next !== idx) {
      next = forward
        ? (next + 1) % items.length
        : (next - 1 + items.length) % items.length;
    }
    const target = items[next];
    if (!target || target.disabled) return;
    onChange(target.id);
    refs.current[target.id]?.focus();
  }

  return (
    <div
      role="tablist"
      aria-label={label}
      className={`${styles.list} ${styles[variant]}`}
      onKeyDown={onKeyDown}
    >
      {items.map((tab) => {
        const selected = tab.id === value;
        return (
          <button
            key={tab.id}
            ref={(el) => {
              refs.current[tab.id] = el;
            }}
            role="tab"
            type="button"
            id={`tab-${tab.id}`}
            aria-selected={selected}
            aria-controls={`panel-${tab.id}`}
            aria-label={tab.ariaLabel}
            tabIndex={selected ? 0 : -1}
            disabled={tab.disabled}
            className={styles.tab}
            onClick={() => onChange(tab.id)}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}

export function TabPanel({
  id,
  active,
  children,
}: {
  id: string;
  active: boolean;
  children: ReactNode;
}) {
  return (
    <div
      role="tabpanel"
      id={`panel-${id}`}
      aria-labelledby={`tab-${id}`}
      hidden={!active}
      tabIndex={0}
      className={styles.panel}
    >
      {active ? children : null}
    </div>
  );
}
