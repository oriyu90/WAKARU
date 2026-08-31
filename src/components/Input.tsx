import { forwardRef } from "react";
import type { InputHTMLAttributes } from "react";
import styles from "./controls.module.css";

export type InputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "className">;

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  props,
  ref,
) {
  return <input ref={ref} className={styles.input} {...props} />;
});
