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
import { Button, Field, Input, StatusText, ToolbarButton } from "../../../ui";

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
      <div className="ac-control-group" role="search" aria-label="goto offset">
        <Field label="Goto offset" htmlFor="hex-goto-box" className="min-w-0 flex-1">
          <Input
            id="hex-goto-box"
            data-testid="hex-goto-input"
            controlSize="sm"
            value={value}
            onChange={(event) => onValueChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") onGo();
              else if (event.key === "Escape") onClose();
            }}
            placeholder="0x10 or 16"
            autoComplete="off"
            spellCheck={false}
            className="ac-code min-w-32 flex-1"
          />
        </Field>
        <Button
          data-testid="hex-goto-go"
          size="sm"
          onClick={onGo}
        >
          Go
        </Button>
        <ToolbarButton
          data-testid="hex-goto-close"
          label="Dismiss goto (Esc)"
          title="Dismiss goto (Esc)"
          icon={X}
          onClick={onClose}
        />
      </div>
      <StatusText
        data-testid="hex-goto-status"
        role="status"
        className="block pt-[var(--ac-space-1)]"
      >
        {statusText}
      </StatusText>
    </section>
  );
}
