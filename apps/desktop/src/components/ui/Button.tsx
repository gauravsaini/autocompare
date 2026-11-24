import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly variant?: ButtonVariant;
  readonly size?: ButtonSize;
  readonly children: ReactNode;
}

const base =
  "inline-flex items-center justify-center gap-1 rounded-[5px] font-medium ac-chrome-label " +
  "border transition focus-visible:outline-hidden focus-visible:ring-2 " +
  "focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45";

const sizes: Record<ButtonSize, string> = {
  sm: "h-[var(--ac-control-h-sm)] px-2 text-[12px]",
  md: "h-[var(--ac-control-h-md)] px-2.5 text-[12px]",
};

const variants: Record<ButtonVariant, string> = {
  primary:
    "border-transparent bg-[var(--ac-btn-primary-bg)] text-[var(--ac-btn-primary-fg)] hover:brightness-105 active:brightness-95 shadow-[var(--ac-btn-shadow)]",
  secondary:
    "border-[var(--ac-btn-border)] bg-[var(--ac-btn-bg)] text-[var(--ac-btn-fg)] hover:bg-[var(--ac-btn-hover-bg)] active:bg-[var(--ac-btn-active-bg)] shadow-[var(--ac-btn-shadow)]",
  ghost:
    "border-transparent bg-transparent text-[var(--ac-fg)] hover:bg-[var(--ac-btn-hover-bg)] active:bg-[var(--ac-btn-active-bg)]",
  danger:
    "border-transparent bg-[var(--ac-btn-danger-bg)] text-[var(--ac-btn-danger-fg)] hover:brightness-105 active:brightness-95 shadow-[var(--ac-btn-shadow)]",
};

/** Shared desktop button: 4 variants, 2 macOS sizes. Testids pass through untouched. */
export default function Button({ variant = "secondary", size = "sm", className = "", children, ...rest }: ButtonProps): React.JSX.Element {
  return (
    <button type="button" className={`${base} ${sizes[size]} ${variants[variant]} ${className}`} {...rest}>
      {children}
    </button>
  );
}
