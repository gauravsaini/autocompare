/**
 * Wave-66R: ignore options live in the toolbar menu (Beyond Compare
 * style). Collapsible fieldset, open by default for backward compat
 * (wave-30 / wave-66T specs drive the same checkboxes); the toolbar
 * Ignore button toggles it. Purely presentational: ws/case state and
 * the re-compare probe live in `TextView` (wave-30 logic byte-identical,
 * wave-97 without the retired "not supported" note — the Rust
 * `TextOptions` wire honors both flags, echoed as `appliedOptions`).
 */
import { Checkbox } from "../../../ui";

export interface IgnoreMenuProps {
  readonly open: boolean;
  readonly ws: boolean;
  readonly ignoreCase: boolean;
  readonly busy: boolean;
  readonly onChange: (nextWs: boolean, nextCase: boolean) => void;
}

export default function IgnoreMenu(props: IgnoreMenuProps): React.JSX.Element | null {
  const { open, ws, ignoreCase, busy, onChange } = props;
  if (!open) return null;
  return (
    <fieldset
      data-testid="textview/ignore-options"
      className="tv-controls-panel flex flex-wrap items-center gap-x-[var(--ac-space-4)] gap-y-[var(--ac-space-1)] rounded-md border border-[var(--ac-border-subtle)] px-[var(--ac-space-3)] py-[var(--ac-space-2)]"
    >
      <legend className="px-1 ac-chrome-label font-medium text-[var(--ac-fg-muted)]">
        Ignore options
      </legend>
      <Checkbox data-testid="textview/ignore-whitespace" label="Ignore whitespace" checked={ws} disabled={busy} onChange={(e) => onChange(e.target.checked, ignoreCase)} />
      <Checkbox data-testid="textview/ignore-case" label="Ignore case" checked={ignoreCase} disabled={busy} onChange={(e) => onChange(ws, e.target.checked)} />
    </fieldset>
  );
}
