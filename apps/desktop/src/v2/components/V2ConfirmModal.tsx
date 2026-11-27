import React, { useEffect } from "react";
import V2Modal from "./V2Modal";

export interface V2ConfirmModalProps {
  readonly open: boolean;
  readonly title: string;
  readonly subtitle?: string;
  readonly message: string;
  readonly confirmLabel?: string;
  readonly cancelLabel?: string;
  readonly destructive?: boolean;
  readonly onConfirm: () => void;
  readonly onCancel: () => void;
}

export default function V2ConfirmModal({
  open,
  title,
  subtitle,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  destructive = false,
  onConfirm,
  onCancel,
}: V2ConfirmModalProps): React.JSX.Element {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        onConfirm();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, onConfirm]);

  return (
    <V2Modal
      open={open}
      title={title}
      subtitle={subtitle}
      widthClass="max-w-sm"
      onClose={onCancel}
      footer={
        <>
          <button
            type="button"
            data-testid="v2/confirm-cancel"
            onClick={onCancel}
            className="v2-btn"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            data-testid="v2/confirm-accept"
            autoFocus
            onClick={onConfirm}
            className={
              destructive
                ? "v2-btn border-[var(--v2-del-fg)] bg-[var(--v2-del-fg)] text-white hover:opacity-90"
                : "v2-btn v2-btn-primary"
            }
          >
            {confirmLabel}
          </button>
        </>
      }
    >
      <p data-testid="v2/confirm-message" className="text-[12px] leading-[18px] text-[var(--v2-fg-secondary)]">
        {message}
      </p>
    </V2Modal>
  );
}
