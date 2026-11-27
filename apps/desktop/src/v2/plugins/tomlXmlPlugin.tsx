import React, { useEffect, useMemo, useState } from "react";
import { FileCode2 } from "lucide-react";
import type { ComparatorViewProps } from "./registry";
import { defineExternalComparatorPlugin, type ExternalPluginManifest } from "./sdk";
import { diffTomlOrXml, type ConfigFormat } from "../utils/tomlXmlDiff";

const SAMPLE_LEFT_TOML = `[package]
name = "autocompare"
version = "0.5.0"

[server]
host = "127.0.0.1"
port = 3080
`;

const SAMPLE_RIGHT_TOML = `[package]
name = "autocompare"
version = "0.6.0"

[server]
host = "127.0.0.1"
port = 8080
tls = true
`;

export const tomlXmlPluginManifest: ExternalPluginManifest = {
  id: "toml-xml",
  name: "TOML / XML Diff",
  version: "1.0.0",
  description: "Read-only key-path diff for TOML & XML configs",
  extensions: [".toml", ".xml"],
  capabilities: ["read_only"],
  accent: "#0d9488",
  priority: 15,
  keywords: ["toml", "xml", "cargo", "config"],
};

const V2TomlXmlCompare: React.FC<ComparatorViewProps> = ({
  onRegisterCommands,
  onStatusChange,
}) => {
  const [format, setFormat] = useState<ConfigFormat>("toml");
  const [leftText, setLeftText] = useState(SAMPLE_LEFT_TOML);
  const [rightText, setRightText] = useState(SAMPLE_RIGHT_TOML);
  const [onlyDiffs, setOnlyDiffs] = useState(false);

  const result = useMemo(
    () => diffTomlOrXml(leftText, rightText, format),
    [leftText, rightText, format],
  );

  const visibleRows = useMemo(
    () => (onlyDiffs ? result.rows.filter((r) => r.kind !== "same") : result.rows),
    [result.rows, onlyDiffs],
  );

  useEffect(() => {
    onRegisterCommands({});
    onStatusChange({
      left: `${format.toUpperCase()} Config Diff (SDK · read_only)`,
      center: `${result.changedCount} changed · ${result.addedCount} added · ${result.removedCount} removed`,
      right: `${result.rows.length} keys`,
    });
  }, [format, result, onRegisterCommands, onStatusChange]);

  return (
    <div data-testid="v2/toml-xml-view" className="flex h-full flex-col bg-[var(--v2-bg)] text-[12px]">
      <div className="flex items-center gap-2 border-b border-[var(--v2-border)] px-3 py-1.5">
        <span className="font-semibold text-[var(--v2-fg)]">TOML / XML Config Diff</span>
        <span className="rounded bg-teal-500/15 px-1.5 py-0.5 text-[10px] font-medium text-teal-600 dark:text-teal-300">
          SDK · read_only
        </span>
        <select
          aria-label="Config format"
          value={format}
          onChange={(e) => setFormat(e.target.value as ConfigFormat)}
          className="rounded border border-[var(--v2-border)] bg-[var(--v2-surface)] px-2 py-0.5 text-[11px]"
        >
          <option value="toml">TOML</option>
          <option value="xml">XML</option>
        </select>
        <label className="ml-2 flex items-center gap-1 text-[11px] text-[var(--v2-fg-muted)]">
          <input
            type="checkbox"
            checked={onlyDiffs}
            onChange={(e) => setOnlyDiffs(e.target.checked)}
          />
          Only differences
        </label>
      </div>

      <div className="grid grid-cols-2 gap-2 border-b border-[var(--v2-border)] p-2">
        <textarea
          aria-label="Left config"
          value={leftText}
          onChange={(e) => setLeftText(e.target.value)}
          className="h-32 w-full rounded border border-[var(--v2-border)] bg-[var(--v2-surface)] p-2 font-mono text-[11px]"
        />
        <textarea
          aria-label="Right config"
          value={rightText}
          onChange={(e) => setRightText(e.target.value)}
          className="h-32 w-full rounded border border-[var(--v2-border)] bg-[var(--v2-surface)] p-2 font-mono text-[11px]"
        />
      </div>

      <div className="flex-1 overflow-auto p-2">
        <table className="w-full border-collapse font-mono text-[11px]">
          <thead>
            <tr className="border-b border-[var(--v2-border)] text-left text-[var(--v2-fg-muted)]">
              <th className="px-2 py-1">Key Path</th>
              <th className="px-2 py-1">Status</th>
              <th className="px-2 py-1">Left</th>
              <th className="px-2 py-1">Right</th>
            </tr>
          </thead>
          <tbody>
            {visibleRows.map((row) => (
              <tr key={row.path} className="border-b border-[var(--v2-border)]/50">
                <td className="px-2 py-1 font-medium">{row.path}</td>
                <td className="px-2 py-1">{row.kind}</td>
                <td className="px-2 py-1">{row.leftValue ?? "—"}</td>
                <td className="px-2 py-1">{row.rightValue ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};

export const tomlXmlPlugin = defineExternalComparatorPlugin(
  tomlXmlPluginManifest,
  V2TomlXmlCompare,
  FileCode2,
);
