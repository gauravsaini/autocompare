/**
 * Wave-234: goto migrated to ui/DockedBar.
 * Wave-WC: styling-only density pin (DockedBar contract kept byte-identical:
 * in-flow section, Esc + close button ride the single onClose; Go explicit
 * sm=22, input default 24). No testid / Esc-contract change.
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
      <Button size="sm" data-testid="textview/goto-go" variant="primary" onClick={onGo} disabled={busy}>
        {busy ? "Jumping…" : "Go"}
      </Button>
      <StatusText data-testid="textview/goto-status">
        {statusText}
      </StatusText>
    </DockedBar>
  );
}
