import { forwardRef } from "react";
import type { InputHTMLAttributes } from "react";
import styles from "./controls.module.css";

export type SliderProps = Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type" | "className"
>;

export const Slider = forwardRef<HTMLInputElement, SliderProps>(function Slider(
  props,
  ref,
) {
  return <input ref={ref} type="range" className={styles.slider} {...props} />;
});
