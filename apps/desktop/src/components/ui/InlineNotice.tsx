import type { ReactNode } from "react";

export interface InlineNoticeProps {
  readonly testid?: string;
  readonly children: ReactNode;
}

/**
 * Wave-232: the single inline-notice primitive.
 * Exactly one live region (`role=status`); never `role=alert`, so
 * honest parked notes never shout. Surfaces ride existing tokens.
 */
export default function InlineNotice({ testid, children }: InlineNoticeProps): React.JSX.Element {
  return (
    <div
      role="status"
      data-testid={testid}
      className="w-full rounded-[5px] border border-[var(--ac-border-subtle)] bg-[var(--ac-bg-raised)] px-3 py-2 ac-chrome-label text-[var(--ac-fg)]"
    >
      {children}
    </div>
  );
}
