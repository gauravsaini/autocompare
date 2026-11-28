import React, { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { settingsGet, settingsSet } from "../../ipc/session";
import type { AppSettings } from "../../ipc/session";
import { loadKeymapPreset, normalizeKeymapPreset, saveKeymapPreset } from "../utils/keymap";
import type { KeymapPreset } from "../utils/keymap";
import {
  isRestoreLastSessionEnabled,
  loadFileWatchMode,
  saveFileWatchMode,
  setRestoreLastSessionEnabled,
} from "../utils/sessionPrefs";
import type { FileWatchMode } from "../utils/sessionPrefs";
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
  const [keymap, setKeymap] = useState<KeymapPreset>(() => loadKeymapPreset());
  const [fileWatchMode, setFileWatchMode] = useState<FileWatchMode>(() => loadFileWatchMode());
  const [showLineNos, setShowLineNos] = useState(true);
  const [autoSave, setAutoSave] = useState(false);
  const [restoreOnStartup, setRestoreOnStartup] = useState(() => isRestoreLastSessionEnabled());
  const [saving, setSaving] = useState(false);
  const [updateNote, setUpdateNote] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setNote(null);
    setUpdateNote(null);
    setFileWatchMode(loadFileWatchMode());
    setRestoreOnStartup(isRestoreLastSessionEnabled());
    void (async () => {
      try {
        const s = await settingsGet();
        setSettings(s);
        if (typeof s.font_size === "number") setFontSize(s.font_size);
        if (typeof s.tab_size === "number") setTabSize(s.tab_size);
        if (typeof s.keymap === "string") setKeymap(normalizeKeymapPreset(s.keymap));
        if (typeof s.show_line_numbers === "boolean") setShowLineNos(s.show_line_numbers);
        if (typeof s.auto_save === "boolean") setAutoSave(s.auto_save);
      } catch {
        // Keep defaults
      }
    })();
  }, [open]);

  const handleCheckUpdates = (): void => {
    const hasTauri =
      typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
    if (!hasTauri) {
      setUpdateNote("Running in browser preview — auto-updater checks run inside the packaged desktop app.");
      return;
    }
    setUpdateNote("AutoCompare v0.5.0 — configure TAURI_SIGNING_PRIVATE_KEY & updater endpoint for automatic OTA updates.");
  };

  const handleSave = async (): Promise<void> => {
    setSaving(true);
    setNote(null);
    try {
      setRestoreLastSessionEnabled(restoreOnStartup);
      saveKeymapPreset(keymap);
      saveFileWatchMode(fileWatchMode);
      await settingsSet({
        theme,
        keymap,
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
          <div className="inline-flex rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] p-0.5">
            {(["light", "dark"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => onThemeChange(t)}
                className={`h-7 rounded-lg px-3 text-[11.5px] font-medium capitalize transition-colors ${
                  theme === t
                    ? "bg-[var(--v2-accent-fill)] text-[var(--v2-accent-fg)] shadow-sm"
                    : "text-[var(--v2-fg-muted)] hover:text-[var(--v2-fg)]"
                }`}
              >
                {t}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center justify-between gap-3">
          <span className="font-medium text-[var(--v2-fg)]">Keymap Preset</span>
          <select
            aria-label="Keymap Preset"
            value={keymap}
            onChange={(e) => setKeymap(normalizeKeymapPreset(e.target.value))}
            className="v2-input h-[26px] w-44 px-2 text-[11.5px]"
          >
            <option value="default">Default (AutoCompare)</option>
            <option value="bc">Beyond Compare (F7/Shift+F7)</option>
            <option value="vim">Vim (j/k, /, l/h)</option>
            <option value="emacs">Emacs (Ctrl+N/P, Ctrl+S)</option>
          </select>
        </div>

        <div className="flex items-center justify-between gap-3">
          <span className="font-medium text-[var(--v2-fg)]">External File Changes</span>
          <div className="inline-flex rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] p-0.5">
            {(
              [
                ["prompt", "Prompt"],
                ["auto", "Auto-reload"],
                ["off", "Off"],
              ] as const
            ).map(([m, label]) => (
              <button
                key={m}
                type="button"
                onClick={() => setFileWatchMode(m)}
                className={`h-7 rounded-lg px-2.5 text-[11px] font-medium transition-colors ${
                  fileWatchMode === m
                    ? "bg-[var(--v2-accent-fill)] text-[var(--v2-accent-fg)] shadow-sm"
                    : "text-[var(--v2-fg-muted)] hover:text-[var(--v2-fg)]"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <span className="mb-1 block text-[11px] text-[var(--v2-fg-muted)]">Code Font Size (px)</span>
            <input
              type="number"
              aria-label="Code Font Size (px)"
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
              aria-label="Tab Width"
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

        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={restoreOnStartup}
            onChange={(e) => setRestoreOnStartup(e.target.checked)}
            className="accent-[var(--v2-accent)]"
          />
          <span>Restore last comparison tab on startup</span>
        </label>

        <div className="flex items-center justify-between rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] px-3 py-2.5">
          <div className="text-[11px] text-[var(--v2-fg-muted)]">
            {settings?.grammar_preset_name ? (
              <>
                Grammar: <strong className="text-[var(--v2-fg)]">{settings.grammar_preset_name}</strong> ·{" "}
              </>
            ) : null}
            <span>v0.5.0</span>
          </div>
          <button
            type="button"
            onClick={handleCheckUpdates}
            className="v2-btn h-[21px] px-2 text-[10.5px]"
          >
            <RefreshCw size={10} />
            Check for Updates
          </button>
        </div>

        {updateNote ? (
          <p className="text-[11px] text-[var(--v2-fg-secondary)]">{updateNote}</p>
        ) : null}

        {note ? (
          <p className="text-[11px]" style={{ color: "var(--v2-del-fg)" }}>
            {note}
          </p>
        ) : null}
      </div>
    </V2Modal>
  );
}
