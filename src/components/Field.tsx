import { useId } from "react";
import type { ReactNode } from "react";
import { AlertIcon } from "../app/Icons";
import styles from "./controls.module.css";

export type FieldProps = {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  required?: boolean;
  children: (ids: { id: string; describedBy: string; invalid: boolean }) => ReactNode;
};

/** Label + control + one reserved helper line (helper is replaced by error, never
 * shown alongside it — no vertical jump on validation). */
export function Field({ label, hint, error, required, children }: FieldProps) {
  const id = useId();
  const describedBy = `${id}-hint`;
  const invalid = Boolean(error);
  return (
    <div className={styles.field}>
      <label className={styles.fieldLabel} htmlFor={id}>
        {label}
        {required && (
          <span aria-hidden="true" className={styles.fieldRequired}>
            {" *"}
          </span>
        )}
      </label>
      {children({ id, describedBy, invalid })}
      <span
        id={describedBy}
        className={`${styles.fieldHint} ${invalid ? styles.error : ""}`}
      >
        {invalid ? (
          <>
            <AlertIcon size={13} />
            {error}
          </>
        ) : (
          hint
        )}
      </span>
    </div>
  );
}
