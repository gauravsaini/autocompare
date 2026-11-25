/**
 * Wave-199c: ignore options ride the chrome overlay stack (Beyond
 * Compare style popover).
 *
 * Absolute popover (never in-flow): floats over the rows via the
 * `tv-chrome` relative wrapper, so an open ignore panel never pushes
 * `textview/row-0` down. `data-overlay` marks this popover as
 * floating by design (e2e overlap gate skips it explicitly).
 * Purely presentational: ws/case state and the re-compare probe live
 * in `TextView` (wave-30 logic byte-identical, wave-97 without the
 * retired "not supported" note). All testids unchanged.
 * Wave-C2a: engine-ready ignore-blank-lines checkbox
 * (`textview/ignore-blank-lines`, optional props for backward compat).
 * DEBTC: back to popover from the wave-233 fullscreen Modal — the
 * Modal scrim covers the toolbar toggle, so toggle-close (pinned by
 * textinline-192 + textchrome-112/89 + textcontrols-66r) is
 * unclickable while open. Closing rides the toolbar toggle
 * (`aria-expanded`) + the global TextView Escape collapse, same as
 * pre-233; the `onClose` prop stays for caller compat.
 */
import { useState } from "react";
import { Checkbox } from "../../../ui";

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
  const { open, ws, ignoreCase, busy, onChange } = props;
  const [localBlank, setLocalBlank] = useState(false);
  const blank = props.blankLines ?? localBlank;
  if (!open) return null;

  const handleBlank = (next: boolean): void => {
    if (props.onBlankChange) {
      props.onBlankChange(next);
    } else {
      setLocalBlank(next);
    }
  };

  return (
    <fieldset
      data-testid="textview/ignore-options"
      data-overlay="true"
      className="tv-controls-panel flex flex-wrap items-center gap-x-[var(--ac-space-4)] gap-y-[var(--ac-space-1)] rounded-md border border-[var(--ac-border-subtle)] px-[var(--ac-space-3)] py-[var(--ac-space-2)] shadow-lg"
    >
      <legend className="px-1 ac-chrome-label font-medium text-[var(--ac-fg-muted)]">
        Ignore options
      </legend>
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
    </fieldset>
  );
}
