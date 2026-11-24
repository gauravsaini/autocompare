import type { ButtonHTMLAttributes } from "react";

/**
 * Wave-158: shared 12px overview-dot primitive (merge conflict ruler).
 * Replaces the ad-hoc `h-3 w-3 Button` override: 12px lives OUTSIDE the
 * 22-26px control system on purpose (ruler affordance, not a control).
 * Accent fill + border mark the active conflict; the inner pip keeps the
 * pre-migration look byte-identical. Testids + aria pass through untouched.
 */
export interface DotButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly active?: boolean;
}

export default function DotButton({ active = false, className = "", type = "button", ...rest }: DotButtonProps): React.JSX.Element {
  return (
    <button
      type={type}
      data-active={active ? "true" : undefined}
      className={
        "inline-flex h-3 w-3 shrink-0 items-center justify-center rounded-full border transition " +
        "hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 " +
        "focus-visible:ring-[var(--ac-focus-ring)] " +
        (active
          ? "border-[var(--ac-accent)] bg-[var(--ac-accent)] "
          : "border-[var(--ac-border)] bg-[var(--ac-bg)] ") +
        className
      }
      {...rest}
    >
      <span
        aria-hidden="true"
        className="block h-1 w-1 rounded-full"
        style={{ backgroundColor: active ? "var(--ac-bg)" : "var(--ac-fg-muted)" }}
      />
    </button>
  );
}
