/**
 * Wave-233: Format options migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (textview/format-options, textview/format-encoding,
 * textview/format-eol).
 */
import { Modal, Select } from "../../../ui";

/** Window event carrying the top-bar Format request (BC parity). */
export const VIEW_FORMAT_EVENT = "autocompare:view-format";

/** Dispatch a Format-panel open request (top Toolbar -> TextView). */
export function dispatchViewFormat(): void {
  window.dispatchEvent(new CustomEvent(VIEW_FORMAT_EVENT));
}

/** Encoding `""` = auto (legacy exact wire, option omitted). */
export const FORMAT_AUTO = "";

export interface FormatMenuProps {
  readonly open: boolean;
  readonly encoding: string;
  readonly eol: string;
  readonly busy: boolean;
  readonly onChange: (nextEncoding: string, nextEol: string) => void;
  readonly onClose?: () => void;
}

export default function FormatMenu(props: FormatMenuProps): React.JSX.Element | null {
  const { open, encoding, eol, busy, onChange, onClose } = props;

  const handleClose = (): void => {
    if (onClose) {
      onClose();
      return;
    }
    const btn = document.querySelector<HTMLButtonElement>('[data-testid="textview/toolbar-format"]');
    if (btn && btn.getAttribute("aria-expanded") === "true") {
      btn.click();
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      label="Format options"
      testid="textview/format-options"
    >
      <div className="flex flex-col gap-3 pt-1">
        <label className="flex items-center justify-between gap-2 text-[length:var(--ac-font-size-ui)] font-normal text-[var(--ac-fg)]">
          <span>Encoding</span>
          <Select
            data-testid="textview/format-encoding"
            aria-label="File encoding for compare"
            title="Decode both files with this encoding before diffing (Auto = UTF-8)"
            value={encoding}
            disabled={busy}
            onChange={(e) => onChange(e.target.value, eol)}
          >
            <option value="">Auto (UTF-8)</option>
            <option value="utf-8">UTF-8</option>
            <option value="windows-1252">Windows-1252</option>
            <option value="latin-1">Latin-1</option>
          </Select>
        </label>
        <label className="flex items-center justify-between gap-2 text-[length:var(--ac-font-size-ui)] font-normal text-[var(--ac-fg)]">
          <span>Line endings</span>
          <Select
            data-testid="textview/format-eol"
            aria-label="Line ending policy for compare"
            title="Normalise line endings before diffing (Auto = exact bytes)"
            value={eol}
            disabled={busy}
            onChange={(e) => onChange(encoding, e.target.value)}
          >
            <option value="">Auto (exact)</option>
            <option value="lf">LF</option>
            <option value="crlf">CRLF</option>
            <option value="cr">CR</option>
          </Select>
        </label>
      </div>
    </Modal>
  );
}
