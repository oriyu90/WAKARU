import { forwardRef } from "react";
import type { SelectHTMLAttributes } from "react";
import styles from "./controls.module.css";

export type SelectProps = Omit<SelectHTMLAttributes<HTMLSelectElement>, "className">;

/** Native `<select>` styled through its wrapper — keeps keyboard + SR behaviour. */
export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(
  { children, ...props },
  ref,
) {
  return (
    <select ref={ref} className={styles.select} {...props}>
      {children}
    </select>
  );
});
