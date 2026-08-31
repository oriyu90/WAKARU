import { forwardRef } from "react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import styles from "./controls.module.css";

export type IconButtonProps = Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "className" | "aria-label"
> & {
  /** Required — icon-only buttons must be named for screen readers. */
  label: string;
  size?: "md" | "sm";
  variant?: "secondary" | "quiet";
  disabledReason?: string;
  children: ReactNode;
};

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  function IconButton(
    { label, size = "md", variant = "quiet", disabled, disabledReason, children, type = "button", ...rest },
    ref,
  ) {
    const cls = [
      styles.control,
      styles[variant],
      styles.iconBtn,
      size === "sm" && styles.small,
    ]
      .filter(Boolean)
      .join(" ");
    return (
      <button
        ref={ref}
        type={type}
        className={cls}
        aria-label={label}
        title={disabled && disabledReason ? disabledReason : label}
        disabled={disabled}
        aria-disabled={disabled || undefined}
        {...rest}
      >
        {children}
      </button>
    );
  },
);
