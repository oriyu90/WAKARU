import { forwardRef } from "react";
import type { TextareaHTMLAttributes } from "react";
import styles from "./controls.module.css";

export type TextareaProps = Omit<
  TextareaHTMLAttributes<HTMLTextAreaElement>,
  "className"
>;

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  function Textarea(props, ref) {
    return <textarea ref={ref} className={styles.textarea} {...props} />;
  },
);
