import type { InputHTMLAttributes } from "react";

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  readonly controlSize?: "sm" | "md";
}

/** Uniform 24px input with hairline border + blue focus ring. Testids pass through. */
export default function Input({ controlSize = "sm", className = "", ...rest }: InputProps): React.JSX.Element {
  const h = controlSize === "sm" ? "h-[var(--ac-control-h-sm)]" : "h-[var(--ac-control-h-md)]";
  return (
    <input
      className={`${h} rounded-[5px] border border-[var(--ac-input-border)] bg-[var(--ac-input-bg)] px-2 ac-chrome-label text-[var(--ac-fg)] placeholder:text-[var(--ac-fg-muted)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring-subtle)] ${className}`}
      {...rest}
    />
  );
}
