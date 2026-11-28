import React, { useEffect } from "react";
import { X } from "lucide-react";

export interface V2ModalProps {
  readonly open: boolean;
  readonly title: string;
  readonly subtitle?: string;
  readonly widthClass?: string;
  readonly onClose: () => void;
  readonly footer?: React.ReactNode;
  readonly children: React.ReactNode;
}

/**
 * Single canonical centered Modal standard for AutoCompare v2.
 * Backdrop-dimmed, viewport-centered, Esc-dismissable, zero floating popovers.
 */
export default function V2Modal({
  open,
  title,
  subtitle,
  widthClass = "max-w-lg",
  onClose,
  footer,
  children,
}: V2ModalProps): React.JSX.Element | null {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      data-testid="v2/modal"
      aria-modal="true"
      aria-label={title}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4 backdrop-blur-[3px]"
      onClick={onClose}
    >
      <div
        style={{ boxShadow: "var(--v2-shadow-spotlight)" }}
        className={`w-full ${widthClass} overflow-hidden rounded-2xl border border-[var(--v2-border)] bg-[var(--v2-surface)]`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[var(--v2-border-subtle)] bg-[var(--v2-bg)]/60 px-5 py-3.5">
          <div>
            <h2 className="v2-display text-[14px] font-semibold text-[var(--v2-fg)]">{title}</h2>
            {subtitle ? (
              <p className="mt-0.5 text-[11.5px] text-[var(--v2-fg-muted)]">{subtitle}</p>
            ) : null}
          </div>
          <button
            type="button"
            aria-label="Close modal"
            onClick={onClose}
            className="inline-flex h-7 w-7 items-center justify-center rounded-lg text-[var(--v2-fg-muted)] transition-colors hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
          >
            <X size={15} strokeWidth={1.8} />
          </button>
        </div>

        <div className="max-h-[70vh] overflow-y-auto px-5 py-4">{children}</div>

        {footer ? (
          <div className="flex items-center justify-end gap-2 border-t border-[var(--v2-border-subtle)] bg-[var(--v2-bg)]/60 px-5 py-3">
            {footer}
          </div>
        ) : null}
      </div>
    </div>
  );
}
