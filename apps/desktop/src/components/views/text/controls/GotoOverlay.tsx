/**
 * Wave-234: goto migrated to ui/DockedBar.
 * In-flow collapsible goto bar (never absolute). Purely presentational:
 * input state and paging jump live in TextView.
 * Testids byte-identical.
 */
import { Button, DockedBar, Input, StatusText } from "../../../ui";

export interface GotoOverlayProps {
  readonly open?: boolean;
  readonly value: string;
  readonly placeholder: string;
  readonly onValueChange: (v: string) => void;
  readonly busy: boolean;
  readonly onGo: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function GotoOverlay(props: GotoOverlayProps): React.JSX.Element | null {
  const { open = true, value, placeholder, onValueChange, busy, onGo, onClose, statusText } = props;
  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Goto row"
      testid="textview/goto-panel"
      closeTestId="textview/goto-close"
    >
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
      <StatusText data-testid="textview/goto-status">
        {statusText}
      </StatusText>
    </DockedBar>
  );
}
