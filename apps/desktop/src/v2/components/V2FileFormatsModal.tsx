import React, { useState } from "react";
import { ArrowDown, ArrowUp, Copy, FileCode, Minus, Plus } from "lucide-react";
import V2Modal from "./V2Modal";
import {
  DEFAULT_FILE_FORMATS,
  exportFormatsToToml,
  loadFileFormats,
  saveFileFormats,
  type ConversionKind,
  type FileFormatEntry,
} from "../utils/formatsAndProfiles";

export interface V2FileFormatsModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onSaved?: (msg: string) => void;
}

export default function V2FileFormatsModal({
  open,
  onClose,
  onSaved,
}: V2FileFormatsModalProps): React.JSX.Element {
  const [formats, setFormats] = useState<FileFormatEntry[]>(() => loadFileFormats());
  const [selectedId, setSelectedId] = useState<string>(() => formats[0]?.id ?? "json");
  const [note, setNote] = useState<string | null>(null);

  const selected = formats.find((f) => f.id === selectedId) ?? formats[0];
  const selectedIdx = formats.findIndex((f) => f.id === selected?.id);

  const updateSelected = (patch: Partial<FileFormatEntry>): void => {
    if (!selected) return;
    setFormats((prev) => prev.map((f) => (f.id === selected.id ? { ...f, ...patch } : f)));
  };

  const addFormat = (): void => {
    const id = `custom-${Date.now()}`;
    const next: FileFormatEntry = {
      id,
      name: "Custom Format",
      mask: "*.custom",
      enabled: true,
      conversion: "none",
    };
    setFormats((prev) => [...prev, next]);
    setSelectedId(id);
  };

  const removeSelected = (): void => {
    if (!selected || formats.length <= 1) return;
    const next = formats.filter((f) => f.id !== selected.id);
    setFormats(next);
    setSelectedId(next[Math.max(0, selectedIdx - 1)]?.id ?? next[0].id);
  };

  const moveSelected = (delta: -1 | 1): void => {
    if (selectedIdx < 0) return;
    const target = selectedIdx + delta;
    if (target < 0 || target >= formats.length) return;
    setFormats((prev) => {
      const copy = [...prev];
      const [item] = copy.splice(selectedIdx, 1);
      copy.splice(target, 0, item);
      return copy;
    });
  };

  const handleCopyToml = (): void => {
    const toml = exportFormatsToToml(formats);
    void navigator.clipboard?.writeText(toml).then(
      () => setNote("Copied autocompare.formats.toml to clipboard"),
      () => setNote("Clipboard unavailable"),
    );
  };

  return (
    <V2Modal
      open={open}
      title="File Formats"
      subtitle="Configure extension masks and Phase 3 pre-compare format converters"
      widthClass="max-w-3xl"
      onClose={onClose}
      footer={
        <div className="flex w-full items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => {
                setFormats([...DEFAULT_FILE_FORMATS]);
                setSelectedId(DEFAULT_FILE_FORMATS[0].id);
                setNote("Restored factory default formats");
              }}
              className="v2-btn"
            >
              Factory Defaults
            </button>
            <button type="button" onClick={handleCopyToml} className="v2-btn">
              <Copy size={11} />
              Copy formats.toml
            </button>
            {note ? <span className="text-[11px] text-[var(--v2-accent)]">{note}</span> : null}
          </div>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className="v2-btn">
              Cancel
            </button>
            <button
              type="button"
              onClick={() => {
                saveFileFormats(formats);
                onSaved?.(`Saved ${formats.length} file formats`);
                onClose();
              }}
              className="v2-btn v2-btn-primary"
            >
              Save
            </button>
          </div>
        </div>
      }
    >
      <div data-testid="v2/file-formats-modal" className="grid h-[340px] grid-cols-[240px_1fr] gap-3 text-[12px]">
        {/* Left List: Name | Mask */}
        <div className="flex min-h-0 flex-col rounded-[8px] border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]">
          <div className="grid grid-cols-[20px_1fr_76px] border-b border-[var(--v2-border)] px-2 py-1 text-[10.5px] font-semibold text-[var(--v2-fg-muted)]">
            <span />
            <span>Name</span>
            <span>Mask</span>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto">
            {formats.map((fmt) => {
              const active = fmt.id === selected?.id;
              return (
                <div
                  key={fmt.id}
                  onClick={() => setSelectedId(fmt.id)}
                  className={`grid cursor-pointer grid-cols-[20px_1fr_76px] items-center gap-1 px-2 py-1 text-[11.5px] ${
                    active
                      ? "bg-[var(--v2-accent-soft)] text-[var(--v2-accent)] font-medium"
                      : "hover:bg-[var(--v2-btn-hover)]"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={fmt.enabled}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) =>
                      setFormats((prev) =>
                        prev.map((x) => (x.id === fmt.id ? { ...x, enabled: e.target.checked } : x)),
                      )
                    }
                    className="accent-[var(--v2-accent)]"
                  />
                  <span className="flex items-center gap-1 truncate">
                    <FileCode size={11} className="shrink-0 opacity-70" />
                    <span className="truncate">{fmt.name}</span>
                  </span>
                  <span className="v2-mono truncate text-[10.5px] text-[var(--v2-fg-muted)]">
                    {fmt.mask}
                  </span>
                </div>
              );
            })}
          </div>
          <div className="flex items-center gap-1 border-t border-[var(--v2-border)] p-1.5">
            <button type="button" title="Add format" onClick={addFormat} className="v2-btn h-[22px] px-1.5">
              <Plus size={12} />
            </button>
            <button type="button" title="Remove format" onClick={removeSelected} className="v2-btn h-[22px] px-1.5">
              <Minus size={12} />
            </button>
            <button type="button" title="Move up" onClick={() => moveSelected(-1)} className="v2-btn h-[22px] px-1.5">
              <ArrowUp size={12} />
            </button>
            <button type="button" title="Move down" onClick={() => moveSelected(1)} className="v2-btn h-[22px] px-1.5">
              <ArrowDown size={12} />
            </button>
          </div>
        </div>

        {/* Right Pane: Conversion Details */}
        {selected ? (
          <div className="flex flex-col gap-3 rounded-[8px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-3">
            <div className="grid grid-cols-2 gap-2">
              <label className="block">
                <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                  Format Name
                </span>
                <input
                  type="text"
                  value={selected.name}
                  onChange={(e) => updateSelected({ name: e.target.value })}
                  className="v2-input w-full"
                />
              </label>
              <label className="block">
                <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                  File Mask (semicolon-separated)
                </span>
                <input
                  type="text"
                  value={selected.mask}
                  onChange={(e) => updateSelected({ mask: e.target.value })}
                  className="v2-input v2-mono w-full"
                />
              </label>
            </div>

            <label className="block">
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                Conversion Pipeline
              </span>
              <select
                value={selected.conversion}
                onChange={(e) => updateSelected({ conversion: e.target.value as ConversionKind })}
                className="v2-input w-full"
              >
                <option value="none">None (Compare raw text/bytes)</option>
                <option value="canonical_json">Canonical JSON (Sort object keys recursively)</option>
                <option value="normalize_eol">Normalize Line Endings (CRLF → LF)</option>
                <option value="external">External Read-Only Converter (stdin → stdout, 16 MiB cap)</option>
              </select>
            </label>

            {selected.conversion === "external" ? (
              <div className="grid grid-cols-2 gap-2">
                <label className="block">
                  <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                    Loading Executable
                  </span>
                  <input
                    type="text"
                    value={selected.externalProgram ?? ""}
                    onChange={(e) => updateSelected({ externalProgram: e.target.value })}
                    placeholder="e.g. pdftotext"
                    className="v2-input v2-mono w-full"
                  />
                </label>
                <label className="block">
                  <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                    Arguments
                  </span>
                  <input
                    type="text"
                    value={selected.externalArgs ?? ""}
                    onChange={(e) => updateSelected({ externalArgs: e.target.value })}
                    placeholder="e.g. - -"
                    className="v2-input v2-mono w-full"
                  />
                </label>
              </div>
            ) : null}

            <label className="flex items-center gap-2 text-[11.5px] text-[var(--v2-fg)]">
              <input
                type="checkbox"
                checked={selected.disableEditing === true}
                onChange={(e) => updateSelected({ disableEditing: e.target.checked })}
                className="accent-[var(--v2-accent)]"
              />
              Disable inline editing for converted output (read-only safety)
            </label>

            <p className="mt-auto text-[11px] text-[var(--v2-fg-muted)]">
              Declarative rules export directly to <code className="v2-mono">autocompare.formats.toml</code> and run through the Rust <code className="v2-mono">FormatConverterRegistry</code>.
            </p>
          </div>
        ) : null}
      </div>
    </V2Modal>
  );
}
