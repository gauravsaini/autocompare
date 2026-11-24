/**
 * Wave-66R: collapsible goto bar (Beyond Compare style Ctrl+G overlay).
 *
 * In-flow collapsible (never `position: absolute`, so the overlap
 * gate keeps passing): open by default for backward compat, toggled
 * from the toolbar, Ctrl+G, or Esc. Purely presentational: input
 * state and the paging jump live in `TextView` (wave-43 logic
 * byte-identical).
 */
import { X } from "lucide-react";
import { Button, Input, ToolbarButton } from "../../../ui";

export interface GotoOverlayProps {
  readonly value: string;
  readonly placeholder: string;
  readonly onValueChange: (v: string) => void;
  readonly busy: boolean;
  readonly onGo: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function GotoOverlay(props: GotoOverlayProps): React.JSX.Element {
  const { value, placeholder, onValueChange, busy, onGo, onClose, statusText } = props;
  return (
    <section data-testid="textview/goto-panel" className="tv-controls-panel">
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="goto row">
        <label htmlFor="textview-goto-box" className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          Goto row
        </label>
        <Input
          id="textview-goto-box"
          data-testid="textview/goto-input"
          value={value}
          onChange={(event) => onValueChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") onGo();
            else if (event.key === "Escape") onClose();
          }}
          placeholder={placeholder}
          autoComplete="off"
          spellCheck={false}
          className="ac-mono w-28 text-[length:var(--ac-font-size-code)]"
        />
        <Button data-testid="textview/goto-go" variant="primary" onClick={onGo} disabled={busy}>
          {busy ? "Jumping…" : "Go"}
        </Button>
        <ToolbarButton data-testid="textview/goto-close" label="Dismiss goto (Esc)" title="Dismiss goto (Esc)" onClick={onClose}>
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </ToolbarButton>
      </div>
      <p data-testid="textview/goto-status" className="pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
        {statusText}
      </p>
    </section>
  );
}
