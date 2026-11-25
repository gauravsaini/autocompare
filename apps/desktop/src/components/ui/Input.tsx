import type { InputHTMLAttributes } from "react";

export type InputSize = "sm" | "md";

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  readonly controlSize?: InputSize;
}

/** Uniform desktop input with macOS density (22px/24px), inset hairline border + blue focus ring. Testids pass through. */
export default function Input({ controlSize, className = "", ...rest }: InputProps): React.JSX.Element {
  const h =
    controlSize === "sm"
      ? "h-[var(--ac-control-h-sm)] text-[12px]"
      : controlSize === "md"
        ? "h-[var(--ac-control-h-md)] text-[12px]"
        : "h-[var(--ac-control-h-input,24px)] text-[12px]";
  return (
    <input
      className={`${h} rounded-[5px] border border-[var(--ac-input-border)] bg-[var(--ac-input-bg)] px-2 ac-chrome-label text-[var(--ac-fg)] placeholder:text-[var(--ac-fg-muted)] shadow-[var(--ac-input-shadow,inset_0_0.5px_1px_rgba(0,0,0,0.06))] focus-visible:outline-hidden focus-visible:border-[var(--ac-focus)] focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)] ${className}`}
      {...rest}
    />
  );
}
