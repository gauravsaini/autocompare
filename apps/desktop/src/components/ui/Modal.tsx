import { useEffect, useRef, type ReactNode } from "react";

export interface ModalProps {
  /** `false` renders nothing (no portal, no scrim). */
  readonly open: boolean;
  /** Single dismiss route: Esc, backdrop click, and the close button all call this. */
  readonly onClose: () => void;
  /** Accessible name for `role=dialog`. */
  readonly label: string;
  readonly testid?: string;
  readonly children: ReactNode;
}

/**
 * Wave-232: the single centered modal primitive.
 * Fixed scrim + centered card; surfaces ride existing tokens only
 * (`--ac-bg-raised`, `--ac-border`, `--ac-btn-shadow`, `--ac-focus`).
 * Esc, backdrop mousedown, and the close button all route to `onClose`.
 */
export default function Modal({ open, onClose, label, testid, children }: ModalProps): React.JSX.Element | null {
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    dialogRef.current?.focus();
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
    <div
      data-testid={testid ? `${testid}/backdrop` : undefined}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(0,0,0,0.32)] p-4"
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        tabIndex={-1}
        data-testid={testid}
        className="w-full max-w-md rounded-[8px] border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] p-4 text-[var(--ac-fg)] shadow-[var(--ac-btn-shadow)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus)]"
      >
        <div className="mb-2 flex items-center justify-between gap-2">
          <span className="ac-chrome-label font-medium">{label}</span>
          <button
            type="button"
            data-testid={testid ? `${testid}/close` : undefined}
            aria-label={`Dismiss ${label}`}
            onClick={onClose}
            className="inline-flex h-[var(--ac-control-h-sm)] items-center rounded-[5px] px-2 ac-chrome-label text-[var(--ac-fg-muted)] hover:bg-[var(--ac-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]"
          >
            Dismiss
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
