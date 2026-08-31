export const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

/** Cycle Tab / Shift+Tab within `container`. Call from a keydown listener. */
export function trapTab(container: HTMLElement, event: KeyboardEvent) {
  if (event.key !== "Tab") return;
  const items = Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE),
  ).filter((el) => el.offsetParent !== null || el === document.activeElement);
  if (items.length === 0) return;
  const first = items[0]!;
  const last = items[items.length - 1]!;
  const active = document.activeElement as HTMLElement | null;

  if (event.shiftKey && active === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}
