import React, { useEffect, useState } from "react";
import { settingsGet, settingsSet } from "../../ipc/session";
import type { AppSettings } from "../../ipc/session";
import V2Modal from "./V2Modal";

export interface V2SettingsModalProps {
  readonly open: boolean;
  readonly theme: "light" | "dark";
  readonly onThemeChange: (theme: "light" | "dark") => void;
  readonly onClose: () => void;
}

export default function V2SettingsModal({
  open,
  theme,
  onThemeChange,
  onClose,
}: V2SettingsModalProps): React.JSX.Element {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [fontSize, setFontSize] = useState(12);
  const [tabSize, setTabSize] = useState(4);
  const [showLineNos, setShowLineNos] = useState(true);
  const [autoSave, setAutoSave] = useState(false);
  const [saving, setSaving] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setNote(null);
    void (async () => {
      try {
        const s = await settingsGet();
        setSettings(s);
        if (s.font_size) setFontSize(s.font_size);
        if (s.tab_size) setTabSize(s.tab_size);
        setShowLineNos(s.show_line_numbers);
        setAutoSave(s.auto_save);
      } catch {
        // Keep defaults
      }
    })();
  }, [open]);

  const handleSave = async (): Promise<void> => {
    setSaving(true);
    setNote(null);
    try {
      await settingsSet({
        theme,
        font_size: fontSize,
        tab_size: tabSize,
        show_line_numbers: showLineNos,
        auto_save: autoSave,
      });
      document.documentElement.style.setProperty("--v2-code-font-size", `${fontSize}px`);
      onClose();
    } catch (e) {
      setNote(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <V2Modal
      open={open}
      title="Preferences"
      subtitle="Persisted via Rust settings_set IPC"
      widthClass="max-w-md"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className="v2-btn">
            Cancel
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={() => void handleSave()}
            className="v2-btn v2-btn-primary"
          >
            {saving ? "Saving…" : "Save Preferences"}
          </button>
        </>
      }
    >
      <div className="space-y-3.5 text-[12px]">
        <div className="flex items-center justify-between">
          <span className="font-medium text-[var(--v2-fg)]">Appearance</span>
          <div className="inline-flex rounded-[6px] bg-[var(--v2-seg-track)] p-0.5">
            {(["light", "dark"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => onThemeChange(t)}
                className={`h-[22px] rounded-[5px] px-3 text-[11.5px] font-medium capitalize transition ${
                  theme === t
                    ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                    : "text-[var(--v2-fg-secondary)]"
                }`}
              >
                {t}
              </button>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <span className="mb-1 block text-[11px] text-[var(--v2-fg-muted)]">Code Font Size (px)</span>
            <input
              type="number"
              min={10}
              max={18}
              value={fontSize}
              onChange={(e) => setFontSize(Number(e.target.value) || 12)}
              className="v2-input v2-mono w-full"
            />
          </div>
          <div>
            <span className="mb-1 block text-[11px] text-[var(--v2-fg-muted)]">Tab Width</span>
            <input
              type="number"
              min={2}
              max={8}
              value={tabSize}
              onChange={(e) => setTabSize(Number(e.target.value) || 4)}
              className="v2-input v2-mono w-full"
            />
          </div>
        </div>

        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={showLineNos}
            onChange={(e) => setShowLineNos(e.target.checked)}
            className="accent-[var(--v2-accent)]"
          />
          <span>Show line numbers by default</span>
        </label>

        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={autoSave}
            onChange={(e) => setAutoSave(e.target.checked)}
            className="accent-[var(--v2-accent)]"
          />
          <span>Auto-save edited panes</span>
        </label>

        {settings?.grammar_preset_name ? (
          <div className="rounded-md bg-[var(--v2-surface-subtle)] px-2.5 py-1.5 text-[11px] text-[var(--v2-fg-muted)]">
            Active Grammar Preset: <strong className="text-[var(--v2-fg)]">{settings.grammar_preset_name}</strong>
          </div>
        ) : null}

        {note ? <p className="text-[11px] text-red-600">{note}</p> : null}
      </div>
    </V2Modal>
  );
}
