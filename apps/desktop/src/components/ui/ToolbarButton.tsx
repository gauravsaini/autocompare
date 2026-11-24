import type { ButtonHTMLAttributes, ComponentType, ReactNode } from "react";

export interface ToolbarButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly label: string;
  readonly expanded?: boolean;
  readonly orientation?: "horizontal" | "vertical";
  readonly size?: "sm" | "md";
  readonly icon?: ComponentType<{ size?: number; strokeWidth?: number; className?: string; "aria-hidden"?: boolean | "true" | "false" }>;
  readonly children?: ReactNode;
}

/** Unified icon button: Lucide icons only at call-site (or via icon prop), 14px/1.75 stroke. Testids pass through. */
export default function ToolbarButton({
  label,
  expanded,
  orientation = "horizontal",
  size = "sm",
  icon: Icon,
  className = "",
  children,
  ...rest
}: ToolbarButtonProps): React.JSX.Element {
  const vertical = orientation === "vertical";
  const h = size === "md" ? "h-[var(--ac-control-h-md)] px-2" : "h-[var(--ac-control-h-sm)] px-1.5";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-expanded={expanded}
      data-pressed={expanded === true ? "true" : undefined}
      className={
        (vertical
          ? "inline-flex min-h-[38px] min-w-[44px] flex-col items-center justify-center gap-[var(--ac-space-1)] rounded-[5px] px-[var(--ac-space-2)] py-1 "
          : `inline-flex ${h} items-center gap-1 rounded-[5px] `) +
        "ac-chrome-label font-medium text-[var(--ac-fg)] border border-transparent hover:border-[var(--ac-border-subtle)] hover:bg-[var(--ac-btn-hover-bg)] " +
        "active:bg-[var(--ac-btn-active-bg)] focus-visible:outline-hidden focus-visible:ring-1 " +
        "focus-visible:ring-[var(--ac-focus)] disabled:cursor-not-allowed disabled:opacity-45 select-none " +
        (expanded === true ? "bg-[var(--ac-btn-hover-bg)] border-[var(--ac-border-subtle)] " : "") +
        className
      }
      {...rest}
    >
      {Icon ? <Icon size={14} strokeWidth={1.75} aria-hidden="true" /> : null}
      {children}
    </button>
  );
}
