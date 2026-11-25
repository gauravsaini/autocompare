import { useEffect, type ReactNode } from "react";

export interface DockedBarProps {
  /** `false` renders nothing; when open the bar is in-flow (never absolute/fixed). */
  readonly open: boolean;
  /** Single dismiss route: Esc and the close button both call this. */
  readonly onClose: () => void;
  /** Accessible name for the region. */
  readonly label: string;
  readonly testid?: string;
  readonly closeTestId?: string;
  readonly children: ReactNode;
}

/**
 * Wave-232: the single in-flow collapsible-bar primitive.
 * Replaces per-view find/goto/filter panels (Wave B migrates them here,
 * testids intact). In-flow by construction: no positioning classes, so
 * the overlap gate holds with every panel open.
 */
export default function DockedBar({ open, onClose, label, testid, closeTestId, children }: DockedBarProps): React.JSX.Element | null {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [open, onClose]);

  if (!open) return null;
  return (
    <section
      aria-label={label}
      data-testid={testid}
      className="flex w-full flex-wrap items-center gap-[var(--ac-space-2)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-2)]"
    >
      <span className="ac-chrome-label font-medium text-[var(--ac-fg)]">{label}</span>
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-[var(--ac-space-2)]">{children}</div>
      <button
        type="button"
        data-testid={closeTestId ?? (testid ? `${testid}/close` : undefined)}
        aria-label={`Dismiss ${label}`}
        onClick={onClose}
        className="inline-flex h-[var(--ac-control-h-sm)] items-center rounded-[5px] px-2 ac-chrome-label text-[var(--ac-fg-muted)] hover:bg-[var(--ac-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]"
      >
        Dismiss
      </button>
    </section>
  );
}
