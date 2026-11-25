import type { ReactNode } from "react";

export interface EmptyStateProps {
  readonly title: string;
  readonly body?: string;
  readonly action?: ReactNode;
  readonly testid?: string;
}

/**
 * Wave-232: the single centered empty-state primitive.
 * One heading + one body + one optional action slot, so per-view
 * "nothing to show" blocks converge on one wording/geometry contract.
 */
export default function EmptyState({ title, body, action, testid }: EmptyStateProps): React.JSX.Element {
  return (
    <div data-testid={testid} className="mx-auto flex w-full max-w-md flex-col items-center gap-[var(--ac-space-2)] px-8 py-16 text-center">
      <h2 className="ac-title font-semibold text-[var(--ac-fg)]">{title}</h2>
      {body ? <p className="ac-body text-[var(--ac-fg-muted)]">{body}</p> : null}
      {action}
    </div>
  );
}
