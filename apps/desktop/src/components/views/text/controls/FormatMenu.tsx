/**
 * Wave-164: format options live in the toolbar menu (Beyond Compare
 * style Format button). Collapsible fieldset, closed by default (BC
 * shows one toolbar row + status bar, no stacked panels); the toolbar
 * Format button toggles it. Purely presentational: encoding/eol state
 * and the re-compare probe live in `TextView` (wave-30 ignore logic
 * pattern); the typed `textCompare` wrapper already sends both
 * spellings and the W163c backend honors them, echoed as
 * `appliedOptions`.
 *
 * In-flow collapsible (never `position: absolute`, so the chrome
 * budget holds with the panel open).
 */
import { Select } from "../../../ui";

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
}

export default function FormatMenu(props: FormatMenuProps): React.JSX.Element | null {
  const { open, encoding, eol, busy, onChange } = props;
  if (!open) return null;
  return (
    <fieldset
      data-testid="textview/format-options"
      className="tv-controls-panel flex flex-wrap items-center gap-x-[var(--ac-space-4)] gap-y-[var(--ac-space-1)] rounded-md border border-[var(--ac-border-subtle)] px-[var(--ac-space-3)] py-[var(--ac-space-2)]"
    >
      <legend className="px-1 ac-chrome-label font-medium text-[var(--ac-fg-muted)]">
        Format options
      </legend>
      <label className="inline-flex cursor-pointer items-center gap-2 text-[length:var(--ac-font-size-ui)] font-normal text-[var(--ac-fg)]">
        Encoding
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
      <label className="inline-flex cursor-pointer items-center gap-2 text-[length:var(--ac-font-size-ui)] font-normal text-[var(--ac-fg)]">
        Line endings
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
    </fieldset>
  );
}
