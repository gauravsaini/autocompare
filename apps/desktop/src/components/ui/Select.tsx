import type { SelectHTMLAttributes } from "react";

export type SelectProps = SelectHTMLAttributes<HTMLSelectElement>;

/** Uniform 24px select matching Input. Testids pass through. */
export default function Select({ className = "", children, ...rest }: SelectProps): React.JSX.Element {
  return (
    <select
      className={`h-[var(--ac-control-h-sm)] rounded-[5px] border border-[var(--ac-input-border)] bg-[var(--ac-input-bg)] px-1 ac-chrome-label text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring-subtle)] ${className}`}
      {...rest}
    >
      {children}
    </select>
  );
}
