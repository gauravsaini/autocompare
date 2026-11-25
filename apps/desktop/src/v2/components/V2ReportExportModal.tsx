import React, { useState } from "react";
import { reportExport } from "../../ipc/session";
import type { ReportFormat } from "../../ipc/session";
import type { V2Tab } from "../types";
import V2Modal from "./V2Modal";

export interface V2ReportExportModalProps {
  readonly open: boolean;
  readonly tab: V2Tab;
  readonly onClose: () => void;
}

export default function V2ReportExportModal({
  open,
  tab,
  onClose,
}: V2ReportExportModalProps): React.JSX.Element {
  const [format, setFormat] = useState<ReportFormat>("html");
  const [outPath, setOutPath] = useState(`autocompare-report.${format}`);
  const [exporting, setExporting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleFormatChange = (next: ReportFormat): void => {
    setFormat(next);
    setOutPath((prev) => prev.replace(/\.(html|md|json)$/i, `.${next}`));
  };

  const handleExport = async (): Promise<void> => {
    setExporting(true);
    setError(null);
    setResult(null);
    try {
      const written = await reportExport(
        {
          schema_version: 1,
          name: tab.title,
          locked: false,
          active_tab_index: 0,
          tabs: [
            {
              id: tab.id,
              title: tab.title,
              session_type: tab.kind,
              left_path: tab.leftPath || null,
              right_path: tab.rightPath || null,
              base_path: null,
              output_path: null,
              read_only: false,
              locked: false,
              options: {},
            },
          ],
        },
        format,
        outPath.trim() || `autocompare-report.${format}`,
      );
      setResult(`Exported to ${written}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setExporting(false);
    }
  };

  return (
    <V2Modal
      open={open}
      title="Export Comparison Report"
      subtitle={`Generate a shareable report for "${tab.title}"`}
      widthClass="max-w-md"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className="v2-btn">
            Close
          </button>
          <button
            type="button"
            disabled={exporting}
            onClick={() => void handleExport()}
            className="v2-btn v2-btn-primary"
          >
            {exporting ? "Exporting…" : "Export Report"}
          </button>
        </>
      }
    >
      <div className="space-y-3 text-[12px]">
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">Format</span>
          <div className="inline-flex rounded-[6px] bg-[var(--v2-seg-track)] p-0.5">
            {(
              [
                ["html", "HTML"],
                ["md", "Markdown"],
                ["json", "JSON"],
              ] as const
            ).map(([fmt, label]) => (
              <button
                key={fmt}
                type="button"
                onClick={() => handleFormatChange(fmt)}
                className={`h-[22px] rounded-[5px] px-3 text-[11.5px] font-medium transition ${
                  format === fmt
                    ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                    : "text-[var(--v2-fg-secondary)]"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Output Filename
          </span>
          <input
            type="text"
            value={outPath}
            onChange={(e) => setOutPath(e.target.value)}
            className="v2-input v2-mono w-full"
          />
        </div>

        {result ? (
          <div className="rounded-md bg-[var(--v2-ins-bg)] px-2.5 py-1.5 text-[11.5px] text-[var(--v2-ins-fg)]">
            {result}
          </div>
        ) : null}
        {error ? (
          <div className="rounded-md bg-[var(--v2-del-bg)] px-2.5 py-1.5 text-[11.5px] text-[var(--v2-del-fg)]">
            {error}
          </div>
        ) : null}
      </div>
    </V2Modal>
  );
}
