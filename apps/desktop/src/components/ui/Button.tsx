import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly variant?: ButtonVariant;
  readonly size?: ButtonSize;
  /** Wave-158: full-width menu/option row (PathBar history): w-full + h-auto, no ! overrides. */
  readonly fullWidth?: boolean;
  readonly children: ReactNode;
}

const base =
  "inline-flex items-center justify-center gap-1 rounded-[5px] font-medium ac-chrome-label " +
  "border transition focus-visible:outline-hidden focus-visible:ring-1 " +
  "focus-visible:ring-[var(--ac-focus)] disabled:cursor-not-allowed disabled:opacity-45 select-none";

const sizes: Record<ButtonSize, string> = {
  sm: "h-[var(--ac-control-h-sm)] px-2 text-[12px]",
  md: "h-[var(--ac-control-h-md)] px-2.5 text-[12px]",
};

const variants: Record<ButtonVariant, string> = {
  primary:
    "border-[var(--ac-btn-primary-border,transparent)] bg-[var(--ac-btn-primary-bg)] text-[var(--ac-btn-primary-fg)] " +
    "hover:brightness-105 active:brightness-95 shadow-[var(--ac-btn-primary-shadow,var(--ac-btn-shadow))] " +
    "active:shadow-[var(--ac-btn-primary-active-shadow,none)]",
  secondary:
    "border-[var(--ac-btn-border)] bg-[var(--ac-btn-bg)] text-[var(--ac-btn-fg)] " +
    "hover:bg-[var(--ac-btn-hover-bg)] active:bg-[var(--ac-btn-active-bg)] shadow-[var(--ac-btn-shadow)] " +
    "active:shadow-[var(--ac-btn-active-shadow,none)]",
  ghost:
    "border-transparent bg-transparent text-[var(--ac-fg)] hover:bg-[var(--ac-btn-hover-bg)] active:bg-[var(--ac-btn-active-bg)]",
  danger:
    "border-[var(--ac-btn-danger-border,transparent)] bg-[var(--ac-btn-danger-bg)] text-[var(--ac-btn-danger-fg)] " +
    "hover:brightness-105 active:brightness-95 shadow-[var(--ac-btn-danger-shadow,var(--ac-btn-shadow))] " +
    "active:shadow-[var(--ac-btn-danger-active-shadow,none)]",
};

/** Shared desktop button: 4 variants, 2 macOS sizes (22px/24px), hairline border + bevel. Testids pass through untouched. */
export default function Button({ variant = "secondary", size = "sm", fullWidth = false, className = "", children, ...rest }: ButtonProps): React.JSX.Element {
  const sizeClass = fullWidth
    ? "h-auto w-full justify-start px-2 py-[var(--ac-space-2)] text-[12px]"
    : sizes[size];
  return (
    <button type="button" className={`${base} ${sizeClass} ${variants[variant]} ${className}`} {...rest}>
      {children}
    </button>
  );
}
