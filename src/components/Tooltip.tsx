import { cloneElement, useId, useRef, useState } from "react";
import type { ReactElement, ReactNode } from "react";
import styles from "./Tooltip.module.css";

/** Hover delay 800 ms, focus delay 0 ms (different intents). Content is
 * supplementary — never the only place crucial info lives. */
export function Tooltip({
  content,
  children,
}: {
  content: ReactNode;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  children: ReactElement<any>;
}) {
  const id = useId();
  const [open, setOpen] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const show = (delay: number) => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setOpen(true), delay);
  };
  const hide = () => {
    clearTimeout(timer.current);
    setOpen(false);
  };

  const trigger = cloneElement(children, {
    "aria-describedby": open ? id : undefined,
    onMouseEnter: () => show(800),
    onMouseLeave: hide,
    onFocus: () => show(0),
    onBlur: hide,
  });

  return (
    <span className={styles.wrap}>
      {trigger}
      {open ? (
        <span role="tooltip" id={id} className={styles.tip}>
          {content}
        </span>
      ) : null}
    </span>
  );
}
