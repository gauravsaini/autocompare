import { useEffect, useRef, type ReactNode } from "react";

export interface ModalProps {
  /** `false` renders nothing (no portal, no scrim). */
  readonly open: boolean;
  /** Single dismiss route: Esc, backdrop click, and the close button all call this. */
  readonly onClose: () => void;
  /** Accessible name for `role=dialog`. */
  readonly label: string;
  /** Optional id of element that labels the dialog (overrides or supplements `label`). */
  readonly ariaLabelledby?: string;
  readonly testid?: string;
  readonly children: ReactNode;
  /** Optional custom CSS classes for dialog card container. */
  readonly className?: string;
  /** Optional max width for dialog card, e.g. "max-w-3xl" (defaults to "max-w-md"). */
  readonly maxWidth?: string;
  /** Optional label for the close/dismiss button (defaults to "Dismiss"). */
  readonly closeLabel?: string;
  /** Optional accessible name for close button (defaults to closeLabel ?? `Dismiss ${label}`). */
  readonly closeAriaLabel?: string;
  /** Optional testid for the close button (defaults to `${testid}/close`). */
  readonly closeTestid?: string;
  /** If true, suppresses the default header row so the surface can provide its own header. */
  readonly hideDefaultHeader?: boolean;
}

/**
 * Wave-232: the single centered modal primitive.
 * Fixed scrim + centered card; surfaces ride existing tokens only
 * (`--ac-bg-raised`, `--ac-border`, `--ac-btn-shadow`, `--ac-focus`).
 * Esc, backdrop mousedown, and the close button all route to `onClose`.
 */
export default function Modal({
  open,
  onClose,
  label,
  ariaLabelledby,
  testid,
  children,
  className = "",
  maxWidth = "max-w-md",
  closeLabel,
  closeAriaLabel,
  closeTestid,
  hideDefaultHeader = false,
}: ModalProps): React.JSX.Element | null {
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    if (!dialogRef.current?.contains(document.activeElement)) {
      dialogRef.current?.focus();
    }
    const onKey = (e: KeyboardEvent): void => {
      if (e.defaultPrevented) return;
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [open, onClose]);

  if (!open) return null;

  const cardPadding = hideDefaultHeader ? "" : "p-4";
  const cardClasses = [
    "w-full",
    maxWidth,
    "rounded-[8px] border border-[var(--ac-border)] bg-[var(--ac-bg-raised)]",
    cardPadding,
    "text-[var(--ac-fg)] shadow-[var(--ac-btn-shadow)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus)]",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      data-testid={testid ? `${testid}/backdrop` : undefined}
      data-overlay="true"
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
        aria-labelledby={ariaLabelledby}
        tabIndex={-1}
        data-testid={testid}
        className={cardClasses}
      >
        {!hideDefaultHeader && (
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="ac-chrome-label font-medium">{label}</span>
            <button
              type="button"
              data-testid={closeTestid ?? (testid ? `${testid}/close` : undefined)}
              aria-label={closeAriaLabel ?? closeLabel ?? `Dismiss ${label}`}
              onClick={onClose}
              className="inline-flex h-[var(--ac-control-h-sm)] items-center rounded-[5px] px-2 ac-chrome-label text-[var(--ac-fg-muted)] hover:bg-[var(--ac-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]"
            >
              {closeLabel ?? "Dismiss"}
            </button>
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
