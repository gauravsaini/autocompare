import type { ReactNode } from "react";

/**
 * Wave-158: shared labeled-field primitive (typography ONLY via `ac-meta`,
 * no geometry/color of its own). Replaces ad-hoc
 * `text-[length:var(--ac-font-size-ui)]` label trios (lightningcss rejects
 * them) next to `Input`/`Select` controls. Testids ride the wrapped
 * control, untouched.
 */
export interface FieldProps {
  readonly label: string;
  readonly htmlFor?: string;
  readonly children: ReactNode;
  readonly className?: string;
}

export default function Field({ label, htmlFor, children, className = "" }: FieldProps): React.JSX.Element {
  return (
    <label
      htmlFor={htmlFor}
      className={`ac-meta inline-flex shrink-0 items-center gap-[var(--ac-space-1)] font-normal text-[var(--ac-fg-muted)] ${className}`}
    >
      <span>{label}</span>
      {children}
    </label>
  );
}
