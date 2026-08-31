import { forwardRef } from "react";
import type { InputHTMLAttributes } from "react";
import styles from "./controls.module.css";

export type SwitchProps = Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type" | "className" | "role"
> & { label: string };

/** A switch is a checkbox with a different coat — same a11y contract. */
export const Switch = forwardRef<HTMLInputElement, SwitchProps>(function Switch(
  { label, ...props },
  ref,
) {
  return (
    <span className={styles.switch}>
      <input
        ref={ref}
        type="checkbox"
        role="switch"
        aria-label={label}
        {...props}
      />
      <span className={styles.switchTrack} aria-hidden="true" />
      <span className={styles.switchThumb} aria-hidden="true" />
    </span>
  );
});
