/**
 * Wave-233: Ignore options migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (textview/ignore-options, textview/ignore-whitespace,
 * textview/ignore-case). Wave-C2a adds engine-ready ignore-blank-lines
 * (textview/ignore-blank-lines); optional props so legacy callers keep
 * working byte-identically.
 */
import { useState } from "react";
import { Checkbox, Modal } from "../../../ui";

export interface IgnoreMenuProps {
  readonly open: boolean;
  readonly ws: boolean;
  readonly ignoreCase: boolean;
  readonly busy: boolean;
  readonly onChange: (nextWs: boolean, nextCase: boolean) => void;
  readonly onClose?: () => void;
  /** Wave-C2a: blank-line folding (optional for backward compat). */
  readonly blankLines?: boolean;
  /** Wave-C2a: blank-line toggle handler (optional for backward compat). */
  readonly onBlankChange?: (next: boolean) => void;
}

export default function IgnoreMenu(props: IgnoreMenuProps): React.JSX.Element | null {
  const { open, ws, ignoreCase, busy, onChange, onClose } = props;
  const [localBlank, setLocalBlank] = useState(false);
  const blank = props.blankLines ?? localBlank;

  const handleClose = (): void => {
    if (onClose) {
      onClose();
      return;
    }
    const btn = document.querySelector<HTMLButtonElement>('[data-testid="textview/toolbar-ignore"]');
    if (btn && btn.getAttribute("aria-expanded") === "true") {
      btn.click();
    }
  };

  const handleBlank = (next: boolean): void => {
    if (props.onBlankChange) {
      props.onBlankChange(next);
    } else {
      setLocalBlank(next);
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      label="Ignore options"
      testid="textview/ignore-options"
    >
      <div className="flex flex-col gap-2 pt-1">
        <Checkbox
          data-testid="textview/ignore-whitespace"
          label="Ignore whitespace"
          checked={ws}
          disabled={busy}
          onChange={(e) => onChange(e.target.checked, ignoreCase)}
        />
        <Checkbox
          data-testid="textview/ignore-case"
          label="Ignore case"
          checked={ignoreCase}
          disabled={busy}
          onChange={(e) => onChange(ws, e.target.checked)}
        />
        <Checkbox
          data-testid="textview/ignore-blank-lines"
          label="Ignore blank lines"
          checked={blank}
          disabled={busy}
          onChange={(e) => handleBlank(e.target.checked)}
        />
      </div>
    </Modal>
  );
}
