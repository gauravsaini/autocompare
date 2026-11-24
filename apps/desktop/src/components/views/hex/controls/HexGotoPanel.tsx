/**
 * Wave-71: collapsible hex goto-offset panel (Beyond Compare Ctrl+G style).
 *
 * In-flow collapsible (never `position: absolute`, so the wave-7
 * overlap gate holds): open by default for backward compat (wave-42/66L
 * specs fill the goto box on load), toggled from the toolbar, Ctrl+G, or
 * Esc. Purely presentational: input state and the paging jump live in
 * `HexView` (wave-42 logic byte-identical). All legacy `hex-goto-*`
 * testids ride the same inputs.
 */
import { X } from "lucide-react";
import { Button, Input, ToolbarButton } from "../../../ui";

export interface HexGotoPanelProps {
  readonly value: string;
  readonly onValueChange: (v: string) => void;
  readonly onGo: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function HexGotoPanel(props: HexGotoPanelProps): React.JSX.Element {
  const { value, onValueChange, onGo, onClose, statusText } = props;
  return (
    <section data-testid="hex/goto-panel" className="tv-controls-panel">
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="goto offset">
        <label
          htmlFor="hex-goto-box"
          className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          Goto offset
        </label>
        <Input
          id="hex-goto-box"
          data-testid="hex-goto-input"
          value={value}
          onChange={(event) => onValueChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") onGo();
            else if (event.key === "Escape") onClose();
          }}
          placeholder="0x10 or 16"
          autoComplete="off"
          spellCheck={false}
          className="ac-mono min-w-32 flex-1 text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)]"
        />
        <Button
          data-testid="hex-goto-go"
          onClick={onGo}
        >
          Go
        </Button>
        <ToolbarButton
          data-testid="hex-goto-close"
          label="Dismiss goto (Esc)"
          title="Dismiss goto (Esc)"
          onClick={onClose}
        >
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </ToolbarButton>
      </div>
      <p
        data-testid="hex-goto-status"
        className="pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {statusText}
      </p>
    </section>
  );
}
