/**
 * Wave-233: Ignore options migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (textview/ignore-options, textview/ignore-whitespace,
 * textview/ignore-case).
 */
import { Checkbox, Modal } from "../../../ui";

export interface IgnoreMenuProps {
  readonly open: boolean;
  readonly ws: boolean;
  readonly ignoreCase: boolean;
  readonly busy: boolean;
  readonly onChange: (nextWs: boolean, nextCase: boolean) => void;
  readonly onClose?: () => void;
}

export default function IgnoreMenu(props: IgnoreMenuProps): React.JSX.Element | null {
  const { open, ws, ignoreCase, busy, onChange, onClose } = props;

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
      </div>
    </Modal>
  );
}
