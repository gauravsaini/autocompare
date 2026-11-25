/**
 * Wave-234: hex goto migrated to ui/DockedBar.
 * In-flow collapsible hex goto bar (never absolute). Purely presentational:
 * input state and paging jump live in HexView.
 * Testids byte-identical.
 */
import { Button, DockedBar, Input, StatusText } from "../../../ui";

export interface HexGotoPanelProps {
  readonly open?: boolean;
  readonly value: string;
  readonly onValueChange: (v: string) => void;
  readonly onGo: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function HexGotoPanel(props: HexGotoPanelProps): React.JSX.Element | null {
  const { open = true, value, onValueChange, onGo, onClose, statusText } = props;
  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Goto offset"
      testid="hex/goto-panel"
      closeTestId="hex-goto-close"
    >
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
      <Button
        data-testid="hex-goto-go"
        size="sm"
        onClick={onGo}
      >
        Go
      </Button>
      <StatusText
        data-testid="hex-goto-status"
        role="status"
        className="block"
      >
        {statusText}
      </StatusText>
    </DockedBar>
  );
}
