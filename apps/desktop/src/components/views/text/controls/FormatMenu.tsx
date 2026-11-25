/**
 * Wave-233: Format options migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (textview/format-options, textview/format-encoding,
 * textview/format-eol). Wave-C2a adds engine-ready Patience toggle
 * (textview/algorithm-patience); optional props so legacy callers keep
 * working byte-identically.
 * Wave-WC: styling-only — Modal shape KEPT (centered card, Esc+backdrop+
 * close single onClose); selects ride ui/Select (default 24px) + checkbox
 * 22px byte-identical. No testid / Esc-contract change.
 */
import { useState } from "react";
import { Checkbox, Modal, Select } from "../../../ui";

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
  /** Wave-C2a: `"myers"` default | `"patience"` (optional for backward compat). */
  readonly algorithm?: string;
  /** Wave-C2a: algorithm toggle handler (optional for backward compat). */
  readonly onAlgorithmChange?: (next: string) => void;
}

export default function FormatMenu(props: FormatMenuProps): React.JSX.Element | null {
  const { open, encoding, eol, busy, onChange, onClose } = props;
  const [localAlgo, setLocalAlgo] = useState("myers");
  const algo = props.algorithm ?? localAlgo;
  const patience = algo === "patience";

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

  const handleAlgo = (next: boolean): void => {
    const value = next ? "patience" : "myers";
    if (props.onAlgorithmChange) {
      props.onAlgorithmChange(value);
    } else {
      setLocalAlgo(value);
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
        <Checkbox
          data-testid="textview/algorithm-patience"
          label="Patience diff"
          checked={patience}
          disabled={busy}
          onChange={(e) => handleAlgo(e.target.checked)}
        />
      </div>
    </Modal>
  );
}
