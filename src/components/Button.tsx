import { forwardRef } from "react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Spinner } from "./Spinner";
import { CheckIcon } from "../app/Icons";
import styles from "./controls.module.css";

type Variant = "primary" | "secondary" | "quiet" | "danger";

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: Variant;
  size?: "md" | "sm";
  block?: boolean;
  loading?: boolean;
  /** Show a transient success tick instead of children. Parent controls timing. */
  success?: boolean;
  /** Explain *why* it is disabled — surfaced as the title tooltip. */
  disabledReason?: string;
  icon?: ReactNode;
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = "secondary",
    size = "md",
    block,
    loading = false,
    success = false,
    disabled,
    disabledReason,
    icon,
    children,
    type = "button",
    className,
    ...rest
  },
  ref,
) {
  const isDisabled = disabled || loading;
  const cls = [
    styles.control,
    styles[variant],
    size === "sm" && styles.small,
    block && styles.block,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      ref={ref}
      type={type}
      className={cls}
      disabled={isDisabled}
      aria-disabled={isDisabled || undefined}
      aria-busy={loading || undefined}
      title={disabled && disabledReason ? disabledReason : rest.title}
      {...rest}
    >
      {loading ? (
        <span className={styles.spinnerSlot}>
          <Spinner />
        </span>
      ) : success ? (
        <CheckIcon size={16} />
      ) : (
        icon
      )}
      {!success && <span className={styles.label}>{children}</span>}
    </button>
  );
});
