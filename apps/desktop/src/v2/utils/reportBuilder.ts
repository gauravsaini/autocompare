import type { FolderEntryDto } from "../../ipc/folder";
import type { ReportFormat } from "../../ipc/session";
import type { TablePage } from "../../ipc/table";
import type { TextRowDto } from "../../ipc/text";
import { resolveSyntaxLang, tokenizeLine } from "./syntaxHighlight.ts";
import type { SyntaxLang, SyntaxTokenType } from "./syntaxHighlight.ts";

export interface ReportBuildOptions {
  readonly syntaxHighlight?: boolean;
  readonly collapseUnchanged?: boolean;
  readonly diffsOnly?: boolean;
  readonly contextLines?: number;
}

export interface TextReportData {
  readonly kind: "text";
  readonly title: string;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly rows: readonly TextRowDto[];
  readonly generatedAt?: string;
}

export interface TableReportData {
  readonly kind: "table";
  readonly title: string;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly page: TablePage;
  readonly generatedAt?: string;
}

export interface FolderReportData {
  readonly kind: "folder";
  readonly title: string;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly entries: readonly FolderEntryDto[];
  readonly generatedAt?: string;
}

export interface SummaryReportData {
  readonly kind: "summary";
  readonly sessionType: string;
  readonly title: string;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly generatedAt?: string;
}

export type ReportInput =
  | TextReportData
  | TableReportData
  | FolderReportData
  | SummaryReportData;

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function escapeCsvCell(value: string): string {
  if (/[",\r\n]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

function escapeMdCell(value: string): string {
  return value.replace(/\|/g, "\\|").replace(/\r?\n/g, " ");
}

function tokenClass(type: SyntaxTokenType): string {
  switch (type) {
    case "keyword":
      return "tok-kw";
    case "string":
      return "tok-str";
    case "comment":
      return "tok-cmt";
    case "number":
      return "tok-num";
    case "key":
      return "tok-key";
    case "tag":
      return "tok-tag";
    default:
      return "";
  }
}

function renderHighlightedHtml(line: string, lang: SyntaxLang, enabled: boolean): string {
  if (!enabled || lang === "plain" || line.length === 0) {
    return escapeHtml(line);
  }
  const tokens = tokenizeLine(line, lang);
  let out = "";
  for (const tok of tokens) {
    const slice = escapeHtml(line.slice(tok.start, tok.end));
    const cls = tokenClass(tok.type);
    out += cls ? `<span class="${cls}">${slice}</span>` : slice;
  }
  return out;
}

function formatBytes(bytes?: number | null): string {
  if (bytes === undefined || bytes === null) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const REPORT_CSS = `
:root { color-scheme: light dark; --bg: #ffffff; --fg: #18181b; --muted: #71717a; --border: #e4e4e7; --chrome: #f4f4f5; --del-bg: #fee2e2; --del-fg: #991b1b; --ins-bg: #dcfce7; --ins-fg: #166534; --chg-bg: #fef3c7; --chg-fg: #92400e; --kw: #7c3aed; --str: #047857; --cmt: #71717a; --num: #b45309; --key: #1d4ed8; --tag: #be185d; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #18181b; --fg: #f4f4f5; --muted: #a1a1aa; --border: #27272a; --chrome: #202024; --del-bg: rgba(239,68,68,0.18); --del-fg: #fca5a5; --ins-bg: rgba(34,197,94,0.18); --ins-fg: #86efac; --chg-bg: rgba(245,158,11,0.18); --chg-fg: #fcd34d; --kw: #c4b5fd; --str: #6ee7b7; --cmt: #a1a1aa; --num: #fcd34d; --key: #93c5fd; --tag: #f9a8d4; }
}
* { box-sizing: border-box; }
body { margin: 0; padding: 24px; font: 13px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: var(--bg); color: var(--fg); }
header { border-bottom: 1px solid var(--border); padding-bottom: 14px; margin-bottom: 18px; }
h1 { margin: 0 0 6px; font-size: 18px; }
.meta { font-size: 12px; color: var(--muted); display: flex; flex-wrap: wrap; gap: 12px; }
.stats { margin-top: 10px; display: flex; flex-wrap: wrap; gap: 8px; }
.pill { display: inline-block; padding: 2px 8px; border-radius: 999px; font-size: 11px; font-weight: 600; background: var(--chrome); border: 1px solid var(--border); }
.pill.del { background: var(--del-bg); color: var(--del-fg); }
.pill.ins { background: var(--ins-bg); color: var(--ins-fg); }
.pill.chg { background: var(--chg-bg); color: var(--chg-fg); }
table.diff { width: 100%; border-collapse: collapse; font: 12px/1.45 ui-monospace, SFMono-Regular, Menlo, monospace; border: 1px solid var(--border); }
table.diff th { text-align: left; background: var(--chrome); padding: 6px 8px; border-bottom: 1px solid var(--border); font-family: -apple-system, sans-serif; font-size: 11.5px; }
table.diff td { padding: 2px 8px; border-bottom: 1px solid var(--border); vertical-align: top; white-space: pre-wrap; word-break: break-word; }
td.ln { width: 44px; text-align: right; color: var(--muted); background: var(--chrome); user-select: none; border-right: 1px solid var(--border); }
td.pane { width: 46%; border-right: 1px solid var(--border); }
tr.leftOnly td.left, tr.changed td.left, td.cell-del { background: var(--del-bg); color: var(--del-fg); }
tr.rightOnly td.right, tr.changed td.right, td.cell-ins { background: var(--ins-bg); color: var(--ins-fg); }
details.unchanged-group { margin: 6px 0; border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
details.unchanged-group > summary { cursor: pointer; padding: 5px 10px; background: var(--chrome); color: var(--muted); font-size: 11.5px; font-weight: 500; }
.tok-kw { color: var(--kw); font-weight: 600; }
.tok-str { color: var(--str); }
.tok-cmt { color: var(--cmt); font-style: italic; }
.tok-num { color: var(--num); }
.tok-key { color: var(--key); }
.tok-tag { color: var(--tag); }
@media print { details.unchanged-group { break-inside: avoid; } }
`.trim();

function buildTextHtmlBody(input: TextReportData, opts: ReportBuildOptions): string {
  const syntaxEnabled = opts.syntaxHighlight ?? true;
  const collapse = opts.collapseUnchanged ?? true;
  const diffsOnly = opts.diffsOnly ?? false;
  const lang = resolveSyntaxLang("Detected", input.leftPath || input.rightPath);

  let equalCount = 0;
  let changedCount = 0;
  let leftOnlyCount = 0;
  let rightOnlyCount = 0;

  interface IndexedRow {
    readonly row: TextRowDto;
    readonly leftNum: number | null;
    readonly rightNum: number | null;
  }

  const indexed: IndexedRow[] = [];
  let lNum = 0;
  let rNum = 0;
  for (const row of input.rows) {
    const ln = row.left !== null ? ++lNum : null;
    const rn = row.right !== null ? ++rNum : null;
    if (row.kind === "equal") equalCount += 1;
    else if (row.kind === "changed") changedCount += 1;
    else if (row.kind === "leftOnly") leftOnlyCount += 1;
    else if (row.kind === "rightOnly") rightOnlyCount += 1;
    if (!diffsOnly || row.kind !== "equal") {
      indexed.push({ row, leftNum: ln, rightNum: rn });
    }
  }

  const renderRowsTable = (slice: readonly IndexedRow[], includeHeader: boolean): string => {
    const header = includeHeader
      ? `<thead><tr><th>#</th><th>${escapeHtml(input.leftPath || "Left")}</th><th>#</th><th>${escapeHtml(input.rightPath || "Right")}</th></tr></thead>`
      : "";
    const body = slice
      .map(({ row, leftNum, rightNum }) => {
        const lHtml = row.left !== null ? renderHighlightedHtml(row.left, lang, syntaxEnabled) : "";
        const rHtml = row.right !== null ? renderHighlightedHtml(row.right, lang, syntaxEnabled) : "";
        return `<tr class="${escapeHtml(row.kind)}"><td class="ln">${leftNum ?? ""}</td><td class="pane left">${lHtml}</td><td class="ln">${rightNum ?? ""}</td><td class="pane right">${rHtml}</td></tr>`;
      })
      .join("\n");
    return `<table class="diff">${header}<tbody>${body}</tbody></table>`;
  };

  const chunks: string[] = [];
  if (!collapse || diffsOnly) {
    chunks.push(renderRowsTable(indexed, true));
  } else {
    let i = 0;
    let firstTable = true;
    while (i < indexed.length) {
      if (indexed[i].row.kind === "equal") {
        let j = i;
        while (j < indexed.length && indexed[j].row.kind === "equal") j += 1;
        const run = indexed.slice(i, j);
        if (run.length >= 6) {
          const head = run.slice(0, 2);
          const middle = run.slice(2, -2);
          const tail = run.slice(-2);
          chunks.push(renderRowsTable(head, firstTable));
          firstTable = false;
          chunks.push(
            `<details class="unchanged-group"><summary>Show ${middle.length} unchanged lines</summary>${renderRowsTable(middle, false)}</details>`,
          );
          chunks.push(renderRowsTable(tail, false));
        } else {
          chunks.push(renderRowsTable(run, firstTable));
          firstTable = false;
        }
        i = j;
      } else {
        let j = i;
        while (j < indexed.length && indexed[j].row.kind !== "equal") j += 1;
        chunks.push(renderRowsTable(indexed.slice(i, j), firstTable));
        firstTable = false;
        i = j;
      }
    }
    if (chunks.length === 0) {
      chunks.push(renderRowsTable([], true));
    }
  }

  return `
<div class="stats">
  <span class="pill">Total: ${input.rows.length} rows</span>
  <span class="pill chg">Changed: ${changedCount}</span>
  <span class="pill del">Left only: ${leftOnlyCount}</span>
  <span class="pill ins">Right only: ${rightOnlyCount}</span>
  <span class="pill">Equal: ${equalCount}</span>
</div>
</header>
${chunks.join("\n")}`;
}

function buildTableHtmlBody(input: TableReportData, opts: ReportBuildOptions): string {
  const { page } = input;
  const diffsOnly = opts.diffsOnly ?? false;
  const rowCount = Math.max(page.rowsL.length, page.rowsR.length);
  const colCount = Math.max(page.headerL.length, page.headerR.length, page.totalCols);
  let diffRows = 0;
  const bodyRows: string[] = [];

  for (let r = 0; r < rowCount; r += 1) {
    const rl = page.rowsL[r] ?? [];
    const rr = page.rowsR[r] ?? [];
    let isDiff = rl.length !== rr.length;
    if (!isDiff) {
      for (let c = 0; c < colCount; c += 1) {
        if ((rl[c] ?? "") !== (rr[c] ?? "")) {
          isDiff = true;
          break;
        }
      }
    }
    if (isDiff) diffRows += 1;
    if (diffsOnly && !isDiff) continue;
    const leftCells = Array.from({ length: colCount }, (_, c) => {
      const lv = rl[c] ?? "";
      const rv = rr[c] ?? "";
      const cls = lv !== rv ? ' class="cell-del"' : "";
      return `<td${cls}>${escapeHtml(lv)}</td>`;
    }).join("");
    const rightCells = Array.from({ length: colCount }, (_, c) => {
      const lv = rl[c] ?? "";
      const rv = rr[c] ?? "";
      const cls = lv !== rv ? ' class="cell-ins"' : "";
      return `<td${cls}>${escapeHtml(rv)}</td>`;
    }).join("");
    bodyRows.push(`<tr><td class="ln">${r + 1}</td>${leftCells}${rightCells}</tr>`);
  }

  const headL = Array.from({ length: colCount }, (_, c) => `<th>L: ${escapeHtml(page.headerL[c] ?? `Col ${c + 1}`)}</th>`).join("");
  const headR = Array.from({ length: colCount }, (_, c) => `<th>R: ${escapeHtml(page.headerR[c] ?? `Col ${c + 1}`)}</th>`).join("");

  return `
<div class="stats">
  <span class="pill">Rows: ${rowCount}</span>
  <span class="pill chg">Diff rows: ${diffRows}</span>
  <span class="pill">Columns: ${colCount}</span>
</div>
</header>
<table class="diff">
  <thead><tr><th>#</th>${headL}${headR}</tr></thead>
  <tbody>${bodyRows.join("\n")}</tbody>
</table>`;
}

function buildFolderHtmlBody(input: FolderReportData, opts: ReportBuildOptions): string {
  const diffsOnly = opts.diffsOnly ?? false;
  const filtered = diffsOnly ? input.entries.filter((e) => e.status !== "same") : input.entries;
  const diffCount = input.entries.filter((e) => e.status !== "same").length;
  const rows = filtered
    .map((e) => {
      const cls = e.status === "leftOnly" ? "leftOnly" : e.status === "rightOnly" ? "rightOnly" : e.status === "same" ? "equal" : "changed";
      return `<tr class="${cls}"><td class="pane left">${escapeHtml(e.relPath)}</td><td>${escapeHtml(e.status)}</td><td>${escapeHtml(formatBytes(e.left?.size))}</td><td>${escapeHtml(formatBytes(e.right?.size))}</td></tr>`;
    })
    .join("\n");

  return `
<div class="stats">
  <span class="pill">Entries: ${input.entries.length}</span>
  <span class="pill chg">Differences: ${diffCount}</span>
</div>
</header>
<table class="diff">
  <thead><tr><th>Path</th><th>Status</th><th>Left Size</th><th>Right Size</th></tr></thead>
  <tbody>${rows}</tbody>
</table>`;
}

export function buildHtmlDiffReport(input: ReportInput, opts: ReportBuildOptions = {}): string {
  const ts = input.generatedAt ?? new Date().toISOString();
  let body = "</header><p>Session summary report.</p>";
  if (input.kind === "text") body = buildTextHtmlBody(input, opts);
  else if (input.kind === "table") body = buildTableHtmlBody(input, opts);
  else if (input.kind === "folder") body = buildFolderHtmlBody(input, opts);

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${escapeHtml(input.title)} — AutoCompare Report</title>
<style>${REPORT_CSS}</style>
</head>
<body>
<header>
<h1>${escapeHtml(input.title)}</h1>
<div class="meta">
  <span><strong>Left:</strong> ${escapeHtml(input.leftPath || "(none)")}</span>
  <span><strong>Right:</strong> ${escapeHtml(input.rightPath || "(none)")}</span>
  <span><strong>Generated:</strong> ${escapeHtml(ts)}</span>
</div>${body}
</body>
</html>
`;
}

export function buildUnifiedPatch(input: ReportInput, contextLines = 3): string {
  const leftLabel = input.leftPath || "left";
  const rightLabel = input.rightPath || "right";
  const ctx = Math.max(0, Math.floor(contextLines));

  if (input.kind !== "text") {
    return `--- a/${leftLabel}\n+++ b/${rightLabel}\n# Unified patch is only generated for Text Compare sessions.\n`;
  }

  // Assign 1-based line numbers to every row
  interface Numbered {
    readonly kind: TextRowDto["kind"];
    readonly left: string | null;
    readonly right: string | null;
    readonly lNum: number;
    readonly rNum: number;
  }
  const numbered: Numbered[] = [];
  let lCur = 0;
  let rCur = 0;
  for (const row of input.rows) {
    if (row.left !== null) lCur += 1;
    if (row.right !== null) rCur += 1;
    numbered.push({
      kind: row.kind,
      left: row.left,
      right: row.right,
      lNum: lCur,
      rNum: rCur,
    });
  }

  const diffIndices: number[] = [];
  for (let i = 0; i < numbered.length; i += 1) {
    if (numbered[i].kind !== "equal") diffIndices.push(i);
  }
  if (diffIndices.length === 0) {
    return `--- a/${leftLabel}\n+++ b/${rightLabel}\n`;
  }

  // Group diff indices into hunks within 2*ctx distance
  const ranges: Array<[number, number]> = [];
  for (const idx of diffIndices) {
    const start = Math.max(0, idx - ctx);
    const end = Math.min(numbered.length - 1, idx + ctx);
    const last = ranges[ranges.length - 1];
    if (last && start <= last[1] + 1) {
      last[1] = Math.max(last[1], end);
    } else {
      ranges.push([start, end]);
    }
  }

  const out: string[] = [`--- a/${leftLabel}`, `+++ b/${rightLabel}`];
  for (const [start, end] of ranges) {
    const hunkLines: string[] = [];
    let lStart = 0;
    let rStart = 0;
    let lCount = 0;
    let rCount = 0;

    for (let i = start; i <= end; i += 1) {
      const item = numbered[i];
      if (item.kind === "equal") {
        if (lStart === 0) lStart = item.lNum;
        if (rStart === 0) rStart = item.rNum;
        lCount += 1;
        rCount += 1;
        hunkLines.push(` ${item.left ?? ""}`);
      } else {
        if (item.left !== null) {
          if (lStart === 0) lStart = item.lNum;
          lCount += 1;
          hunkLines.push(`-${item.left}`);
        }
        if (item.right !== null) {
          if (rStart === 0) rStart = item.rNum;
          rCount += 1;
          hunkLines.push(`+${item.right}`);
        }
      }
    }
    out.push(`@@ -${lStart},${lCount} +${rStart},${rCount} @@`);
    out.push(...hunkLines);
  }

  return `${out.join("\n")}\n`;
}

export function buildMarkdownDiffReport(input: ReportInput, opts: ReportBuildOptions = {}): string {
  const diffsOnly = opts.diffsOnly ?? false;
  const lines: string[] = [
    `# ${input.title}`,
    "",
    `- **Left**: \`${input.leftPath || "(none)"}\``,
    `- **Right**: \`${input.rightPath || "(none)"}\``,
    "",
  ];

  if (input.kind === "text") {
    const diffCount = input.rows.filter((r) => r.kind !== "equal").length;
    lines.push(`**Summary**: ${diffCount} differing row(s) out of ${input.rows.length} total.`, "", "```diff");
    const patch = buildUnifiedPatch(input, opts.contextLines ?? 3).trimEnd();
    lines.push(patch, "```", "");
    return lines.join("\n");
  }

  if (input.kind === "table") {
    const { page } = input;
    const rowCount = Math.max(page.rowsL.length, page.rowsR.length);
    const colCount = Math.max(page.headerL.length, page.headerR.length, page.totalCols);
    lines.push("| Row | Status | Left | Right |", "| --- | --- | --- | --- |");
    for (let r = 0; r < rowCount; r += 1) {
      const rl = page.rowsL[r] ?? [];
      const rr = page.rowsR[r] ?? [];
      let isDiff = rl.length !== rr.length;
      if (!isDiff) {
        for (let c = 0; c < colCount; c += 1) {
          if ((rl[c] ?? "") !== (rr[c] ?? "")) {
            isDiff = true;
            break;
          }
        }
      }
      if (diffsOnly && !isDiff) continue;
      lines.push(
        `| ${r + 1} | ${isDiff ? "changed" : "equal"} | ${escapeMdCell(rl.join(", "))} | ${escapeMdCell(rr.join(", "))} |`,
      );
    }
    lines.push("");
    return lines.join("\n");
  }

  if (input.kind === "folder") {
    const entries = diffsOnly ? input.entries.filter((e) => e.status !== "same") : input.entries;
    lines.push("| Path | Status | Left Size | Right Size |", "| --- | --- | --- | --- |");
    for (const e of entries) {
      lines.push(
        `| \`${escapeMdCell(e.relPath)}\` | ${e.status} | ${formatBytes(e.left?.size)} | ${formatBytes(e.right?.size)} |`,
      );
    }
    lines.push("");
    return lines.join("\n");
  }

  lines.push(`Session type: \`${input.sessionType}\``, "");
  return lines.join("\n");
}

export function buildCsvDiffReport(input: ReportInput, opts: ReportBuildOptions = {}): string {
  const diffsOnly = opts.diffsOnly ?? false;
  const rows: string[] = [];

  if (input.kind === "text") {
    rows.push("Row,Status,Left,Right");
    for (const r of input.rows) {
      if (diffsOnly && r.kind === "equal") continue;
      rows.push(
        [
          String(r.index + 1),
          escapeCsvCell(r.kind),
          escapeCsvCell(r.left ?? ""),
          escapeCsvCell(r.right ?? ""),
        ].join(","),
      );
    }
    return `${rows.join("\n")}\n`;
  }

  if (input.kind === "table") {
    const { page } = input;
    const rowCount = Math.max(page.rowsL.length, page.rowsR.length);
    const colCount = Math.max(page.headerL.length, page.headerR.length, page.totalCols);
    const header = [
      "Row",
      "Status",
      ...Array.from({ length: colCount }, (_, c) => escapeCsvCell(`L:${page.headerL[c] ?? `Col ${c + 1}`}`)),
      ...Array.from({ length: colCount }, (_, c) => escapeCsvCell(`R:${page.headerR[c] ?? `Col ${c + 1}`}`)),
    ];
    rows.push(header.join(","));
    for (let r = 0; r < rowCount; r += 1) {
      const rl = page.rowsL[r] ?? [];
      const rr = page.rowsR[r] ?? [];
      let isDiff = rl.length !== rr.length;
      if (!isDiff) {
        for (let c = 0; c < colCount; c += 1) {
          if ((rl[c] ?? "") !== (rr[c] ?? "")) {
            isDiff = true;
            break;
          }
        }
      }
      if (diffsOnly && !isDiff) continue;
      const line = [
        String(r + 1),
        isDiff ? "changed" : "equal",
        ...Array.from({ length: colCount }, (_, c) => escapeCsvCell(rl[c] ?? "")),
        ...Array.from({ length: colCount }, (_, c) => escapeCsvCell(rr[c] ?? "")),
      ];
      rows.push(line.join(","));
    }
    return `${rows.join("\n")}\n`;
  }

  if (input.kind === "folder") {
    rows.push("Path,Status,LeftSize,RightSize");
    for (const e of input.entries) {
      if (diffsOnly && e.status === "same") continue;
      rows.push(
        [
          escapeCsvCell(e.relPath),
          escapeCsvCell(e.status),
          e.left?.size !== undefined ? String(e.left.size) : "",
          e.right?.size !== undefined ? String(e.right.size) : "",
        ].join(","),
      );
    }
    return `${rows.join("\n")}\n`;
  }

  rows.push("Title,Type,Left,Right");
  rows.push(
    [
      escapeCsvCell(input.title),
      escapeCsvCell(input.sessionType),
      escapeCsvCell(input.leftPath),
      escapeCsvCell(input.rightPath),
    ].join(","),
  );
  return `${rows.join("\n")}\n`;
}

export function buildJsonDiffReport(input: ReportInput): string {
  return `${JSON.stringify(
    {
      schemaVersion: 2,
      generatedAt: input.generatedAt ?? new Date().toISOString(),
      ...input,
    },
    null,
    2,
  )}\n`;
}

export function buildReport(
  format: ReportFormat,
  input: ReportInput,
  opts: ReportBuildOptions = {},
): string {
  switch (format) {
    case "html":
      return buildHtmlDiffReport(input, opts);
    case "patch":
      return buildUnifiedPatch(input, opts.contextLines ?? 3);
    case "md":
      return buildMarkdownDiffReport(input, opts);
    case "csv":
      return buildCsvDiffReport(input, opts);
    case "json":
      return buildJsonDiffReport(input);
  }
}
