import type { SelectHTMLAttributes } from "react";

export type SelectSize = "sm" | "md";

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  readonly controlSize?: SelectSize;
}

/** Uniform desktop select with macOS density (22/24/26px), inset hairline border + blue focus ring matching Input. Testids pass through. */
export default function Select({ controlSize, className = "", children, ...rest }: SelectProps): React.JSX.Element {
  const h =
    controlSize === "sm"
      ? "h-[var(--ac-control-h-sm)] text-[12px]"
      : controlSize === "md"
        ? "h-[var(--ac-control-h-md)] text-[12px]"
        : "h-[var(--ac-control-h-input,24px)] text-[12px]";
  return (
    <select
      className={`${h} rounded-[5px] border border-[var(--ac-input-border)] bg-[var(--ac-input-bg)] px-1.5 ac-chrome-label text-[var(--ac-fg)] shadow-[var(--ac-input-shadow,inset_0_0.5px_1px_rgba(0,0,0,0.06))] focus-visible:outline-hidden focus-visible:border-[var(--ac-focus-ring)] focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring-subtle)] ${className}`}
      {...rest}
    >
      {children}
    </select>
  );
}

export { Select };

