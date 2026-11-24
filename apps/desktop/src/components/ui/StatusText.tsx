import type { HTMLAttributes, ReactNode } from "react";

export type StatusTextVariant = "meta" | "code";

export interface StatusTextProps extends HTMLAttributes<HTMLSpanElement> {
  readonly variant?: StatusTextVariant;
  readonly children?: ReactNode;
  readonly className?: string;
}

/**
 * Shared status text primitive.
 * Consolidates ac-meta (11px ui tight) and ac-code (12px mono tabular-nums)
 * typography with text-[var(--ac-fg-muted)].
 */
export function StatusText({
  variant = "meta",
  className = "",
  children,
  ...rest
}: StatusTextProps): React.JSX.Element {
  const typographyClass = variant === "code" ? "ac-code" : "ac-meta";
  return (
    <span
      className={`${typographyClass} text-[var(--ac-fg-muted)] ${className}`}
      {...rest}
    >
      {children}
    </span>
  );
}

export default StatusText;
