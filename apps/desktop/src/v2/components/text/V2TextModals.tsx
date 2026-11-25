import React from "react";
import V2Modal from "../V2Modal";

export interface V2TextRulesModalProps {
  readonly open: boolean;
  readonly ignoreWs: boolean;
  readonly ignoreCase: boolean;
  readonly ignoreBlank: boolean;
  readonly algorithm: "myers" | "patience";
  readonly onIgnoreWsChange: (v: boolean) => void;
  readonly onIgnoreCaseChange: (v: boolean) => void;
  readonly onIgnoreBlankChange: (v: boolean) => void;
  readonly onAlgorithmChange: (v: "myers" | "patience") => void;
  readonly onClose: () => void;
  readonly onApply: () => void;
}

export function V2TextRulesModal({
  open,
  ignoreWs,
  ignoreCase,
  ignoreBlank,
  algorithm,
  onIgnoreWsChange,
  onIgnoreCaseChange,
  onIgnoreBlankChange,
  onAlgorithmChange,
  onClose,
  onApply,
}: V2TextRulesModalProps): React.JSX.Element {
  return (
    <V2Modal
      open={open}
      title="Comparison Rules"
      subtitle="Applies directly to the Rust diff engine"
      widthClass="max-w-md"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className="v2-btn">
            Close
          </button>
          <button type="button" onClick={onApply} className="v2-btn v2-btn-primary">
            Apply & Recompare
          </button>
        </>
      }
    >
      <div className="space-y-3 text-[12px]">
        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={ignoreWs}
            onChange={(e) => onIgnoreWsChange(e.target.checked)}
            className="accent-[var(--v2-accent)]"
          />
          <span>Ignore whitespace changes</span>
        </label>
        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={ignoreCase}
            onChange={(e) => onIgnoreCaseChange(e.target.checked)}
            className="accent-[var(--v2-accent)]"
          />
          <span>Ignore character case</span>
        </label>
        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={ignoreBlank}
            onChange={(e) => onIgnoreBlankChange(e.target.checked)}
            className="accent-[var(--v2-accent)]"
          />
          <span>Ignore blank lines</span>
        </label>
        <div className="pt-1">
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Diff Algorithm
          </span>
          <select
            value={algorithm}
            onChange={(e) => onAlgorithmChange(e.target.value as "myers" | "patience")}
            className="v2-input w-full"
          >
            <option value="myers">Myers (standard)</option>
            <option value="patience">Patience (better block alignment)</option>
          </select>
        </div>
      </div>
    </V2Modal>
  );
}

export interface V2TextFormatModalProps {
  readonly open: boolean;
  readonly encoding: string;
  readonly eol: string;
  readonly onEncodingChange: (v: string) => void;
  readonly onEolChange: (v: string) => void;
  readonly onClose: () => void;
  readonly onApply: () => void;
}

export function V2TextFormatModal({
  open,
  encoding,
  eol,
  onEncodingChange,
  onEolChange,
  onClose,
  onApply,
}: V2TextFormatModalProps): React.JSX.Element {
  return (
    <V2Modal
      open={open}
      title="File Format & Encoding"
      widthClass="max-w-sm"
      onClose={onClose}
      footer={
        <button type="button" onClick={onApply} className="v2-btn v2-btn-primary">
          Apply
        </button>
      }
    >
      <div className="space-y-3 text-[12px]">
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Encoding
          </span>
          <select
            value={encoding}
            onChange={(e) => onEncodingChange(e.target.value)}
            className="v2-input w-full"
          >
            <option value="UTF-8">Unicode (UTF-8)</option>
            <option value="UTF-16">Unicode (UTF-16)</option>
            <option value="ASCII">ASCII</option>
          </select>
        </div>
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Line Endings (EOL)
          </span>
          <select value={eol} onChange={(e) => onEolChange(e.target.value)} className="v2-input w-full">
            <option value="UNIX">UNIX (LF)</option>
            <option value="DOS">Windows (CRLF)</option>
            <option value="MIX">Mixed</option>
          </select>
        </div>
      </div>
    </V2Modal>
  );
}
