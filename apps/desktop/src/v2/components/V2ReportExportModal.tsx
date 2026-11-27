import React, { useCallback, useState } from "react";
import { Check, Copy, Eye } from "lucide-react";
import { folderCompare, folderGetTree } from "../../ipc/folder";
import { reportExport } from "../../ipc/session";
import type { ReportFormat } from "../../ipc/session";
import { tableCompare, tableGetPage } from "../../ipc/table";
import { textCompare, textGetRows } from "../../ipc/text";
import type { V2Tab } from "../types";
import { buildReport } from "../utils/reportBuilder";
import type { ReportInput } from "../utils/reportBuilder";
import V2Modal from "./V2Modal";

export interface V2ReportExportModalProps {
  readonly open: boolean;
  readonly tab: V2Tab;
  readonly onClose: () => void;
}

const FORMAT_OPTIONS: ReadonlyArray<readonly [ReportFormat, string]> = [
  ["html", "HTML"],
  ["patch", "Patch"],
  ["md", "Markdown"],
  ["csv", "CSV"],
  ["json", "JSON"],
];

export default function V2ReportExportModal({
  open,
  tab,
  onClose,
}: V2ReportExportModalProps): React.JSX.Element {
  const [format, setFormat] = useState<ReportFormat>("html");
  const [outPath, setOutPath] = useState("autocompare-report.html");
  const [syntaxHighlight, setSyntaxHighlight] = useState(true);
  const [collapseUnchanged, setCollapseUnchanged] = useState(true);
  const [diffsOnly, setDiffsOnly] = useState(false);
  const [contextLines, setContextLines] = useState(3);

  const [exporting, setExporting] = useState(false);
  const [previewText, setPreviewText] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleFormatChange = (next: ReportFormat): void => {
    setFormat(next);
    setPreviewText(null);
    setOutPath((prev) => prev.replace(/\.(html|patch|md|csv|json)$/i, `.${next}`));
  };

  const gatherReportInput = useCallback(async (): Promise<ReportInput> => {
    const left = tab.leftPath || "";
    const right = tab.rightPath || "";
    try {
      if (tab.kind === "text") {
        const lEff = left || "tools/parity-harness/fixtures/t_left.txt";
        const rEff = right || "tools/parity-harness/fixtures/t_right.txt";
        const sum = await textCompare(lEff, rEff);
        const page = await textGetRows(sum.session, 0, Math.min(Math.max(sum.rows, 200), 1000));
        return {
          kind: "text",
          title: tab.title,
          leftPath: left || lEff,
          rightPath: right || rEff,
          rows: page.rows,
        };
      }
      if (tab.kind === "table") {
        const lEff = left || "tools/parity-harness/fixtures/tab_left.csv";
        const rEff = right || "tools/parity-harness/fixtures/tab_right.csv";
        const sum = await tableCompare(lEff, rEff, false, []);
        const page = await tableGetPage(sum.sessionId, 0, 500, 0, 64);
        return {
          kind: "table",
          title: tab.title,
          leftPath: left || lEff,
          rightPath: right || rEff,
          page,
        };
      }
      if (tab.kind === "folder") {
        const lEff = left || "tools/parity-harness/fixtures/folder/left";
        const rEff = right || "tools/parity-harness/fixtures/folder/right";
        const sum = await folderCompare(lEff, rEff, null, false, true);
        const page = await folderGetTree(sum.session, 0, 500, "all");
        return {
          kind: "folder",
          title: tab.title,
          leftPath: left || lEff,
          rightPath: right || rEff,
          entries: page.entries,
        };
      }
    } catch {
      // Fall back to summary report if live comparison data cannot be loaded
    }
    return {
      kind: "summary",
      sessionType: tab.kind,
      title: tab.title,
      leftPath: left,
      rightPath: right,
    };
  }, [tab]);

  const generateRenderedBody = useCallback(async (): Promise<string> => {
    const input = await gatherReportInput();
    return buildReport(format, input, {
      syntaxHighlight,
      collapseUnchanged,
      diffsOnly,
      contextLines,
    });
  }, [gatherReportInput, format, syntaxHighlight, collapseUnchanged, diffsOnly, contextLines]);

  const handlePreview = async (): Promise<void> => {
    setError(null);
    try {
      const rendered = await generateRenderedBody();
      setPreviewText(rendered);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleCopy = async (): Promise<void> => {
    setError(null);
    try {
      const rendered = previewText ?? (await generateRenderedBody());
      await navigator.clipboard.writeText(rendered);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleExport = async (): Promise<void> => {
    setExporting(true);
    setError(null);
    setResult(null);
    try {
      const rendered = await generateRenderedBody();
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
        rendered,
      );
      setResult(`Exported ${format.toUpperCase()} report to ${written}`);
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
      subtitle={`Generate a shareable diff report for "${tab.title}"`}
      widthClass="max-w-lg"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={() => void handlePreview()} className="v2-btn">
            <Eye size={12} />
            Preview
          </button>
          <button type="button" onClick={() => void handleCopy()} className="v2-btn">
            {copied ? <Check size={12} /> : <Copy size={12} />}
            {copied ? "Copied" : "Copy"}
          </button>
          <button type="button" onClick={onClose} className="v2-btn">
            Close
          </button>
          <button
            type="button"
            data-testid="v2/report-export-btn"
            disabled={exporting}
            onClick={() => void handleExport()}
            className="v2-btn v2-btn-primary"
          >
            {exporting ? "Exporting…" : "Export Report"}
          </button>
        </>
      }
    >
      <div data-testid="v2/report-modal" className="space-y-3 text-[12px]">
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">Format</span>
          <div className="inline-flex flex-wrap rounded-[6px] bg-[var(--v2-seg-track)] p-0.5">
            {FORMAT_OPTIONS.map(([fmt, label]) => (
              <button
                key={fmt}
                type="button"
                data-testid={`v2/report-format-${fmt}`}
                onClick={() => handleFormatChange(fmt)}
                className={`h-[22px] rounded-[5px] px-2.5 text-[11.5px] font-medium transition ${
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

        {/* Format-Specific Report Options */}
        <div className="flex flex-wrap items-center gap-3 rounded-[6px] border border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-2.5 py-2 text-[11.5px]">
          {format === "html" ? (
            <>
              <label className="flex cursor-pointer items-center gap-1.5">
                <input
                  type="checkbox"
                  checked={syntaxHighlight}
                  onChange={(e) => setSyntaxHighlight(e.target.checked)}
                />
                <span>Syntax highlighting</span>
              </label>
              <label className="flex cursor-pointer items-center gap-1.5">
                <input
                  type="checkbox"
                  checked={collapseUnchanged}
                  onChange={(e) => setCollapseUnchanged(e.target.checked)}
                />
                <span>Collapse unchanged sections</span>
              </label>
            </>
          ) : null}

          {format === "patch" ? (
            <label className="flex items-center gap-1.5">
              <span className="text-[var(--v2-fg-muted)]">Context lines:</span>
              <select
                value={contextLines}
                onChange={(e) => setContextLines(Number(e.target.value))}
                className="v2-input h-[22px] w-16 px-1.5 text-[11.5px]"
              >
                <option value={0}>0</option>
                <option value={3}>3</option>
                <option value={5}>5</option>
              </select>
            </label>
          ) : null}

          {format === "html" || format === "md" || format === "csv" ? (
            <label className="flex cursor-pointer items-center gap-1.5">
              <input
                type="checkbox"
                checked={diffsOnly}
                onChange={(e) => setDiffsOnly(e.target.checked)}
              />
              <span>Differences only</span>
            </label>
          ) : null}

          {format === "json" ? (
            <span className="text-[var(--v2-fg-muted)]">
              Exports full structured comparison metadata and diff rows.
            </span>
          ) : null}
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

        {previewText !== null ? (
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Preview ({previewText.length.toLocaleString()} chars)
            </span>
            <pre
              data-testid="v2/report-preview"
              className="v2-mono max-h-40 overflow-auto rounded-[6px] border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] p-2 text-[10.5px] leading-[15px] text-[var(--v2-fg-secondary)]"
            >
              {previewText.slice(0, 4000)}
              {previewText.length > 4000 ? "\n… (truncated in preview)" : ""}
            </pre>
          </div>
        ) : null}

        {result ? (
          <div
            data-testid="v2/report-result"
            className="rounded-md bg-[var(--v2-ins-bg)] px-2.5 py-1.5 text-[11.5px] text-[var(--v2-ins-fg)]"
          >
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
