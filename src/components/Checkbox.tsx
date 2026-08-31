import { forwardRef } from "react";
import type { InputHTMLAttributes, ReactNode } from "react";
import styles from "./controls.module.css";

export type CheckboxProps = Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type" | "className"
> & { label: ReactNode };

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  function Checkbox({ label, ...props }, ref) {
    return (
      <label className={styles.check}>
        <input ref={ref} type="checkbox" {...props} />
        <span>{label}</span>
      </label>
    );
  },
);
