import React from "react";

export interface V2ErrorBannerProps {
  readonly message: string | null;
  readonly onDismiss?: () => void;
}

/**
 * Pure design-token error banner for AutoCompare v2.
 * Uses only var(--v2-*) tokens — zero hardcoded Tailwind palette classes.
 */
export default function V2ErrorBanner({
  message,
  onDismiss,
}: V2ErrorBannerProps): React.JSX.Element | null {
  if (!message) return null;
  return (
    <div
      role="alert"
      data-testid="v2/error-banner"
      style={{
        backgroundColor: "var(--v2-del-bg)",
        color: "var(--v2-del-fg)",
        borderColor: "var(--v2-border)",
      }}
      className="flex shrink-0 items-center justify-between border-b px-3.5 py-2 text-[12px]"
    >
      <span className="truncate font-medium">{message}</span>
      {onDismiss ? (
        <button
          type="button"
          onClick={onDismiss}
          className="ml-3 shrink-0 rounded-lg px-2 py-0.5 text-[11px] font-medium underline opacity-85 hover:opacity-100"
        >
          Dismiss
        </button>
      ) : null}
    </div>
  );
}
