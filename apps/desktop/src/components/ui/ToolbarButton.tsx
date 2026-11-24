import type { ButtonHTMLAttributes, ReactNode } from "react";

export interface ToolbarButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly label: string;
  readonly expanded?: boolean;
  readonly orientation?: "horizontal" | "vertical";
  readonly children: ReactNode;
}

/** Unified icon button: Lucide icons only at call-site, 14px/1.75 stroke. Testids pass through. */
export default function ToolbarButton({ label, expanded, orientation = "horizontal", className = "", children, ...rest }: ToolbarButtonProps): React.JSX.Element {
  const vertical = orientation === "vertical";
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
          : "inline-flex h-[var(--ac-control-h-sm)] items-center gap-1 rounded-[5px] px-1.5 ") +
        "ac-chrome-label font-medium text-[var(--ac-fg)] hover:bg-[var(--ac-btn-hover-bg)] " +
        "active:bg-[var(--ac-btn-active-bg)] focus-visible:outline-hidden focus-visible:ring-2 " +
        "focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45 " +
        (expanded === true ? "bg-[var(--ac-btn-hover-bg)] " : "") + className
      }
      {...rest}
    >
      {children}
    </button>
  );
}
