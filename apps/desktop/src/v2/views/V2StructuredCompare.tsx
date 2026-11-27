import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Braces, Search, Sparkles } from "lucide-react";
import { textCompare, textGetRows } from "../../ipc/text";
import V2DualPathBar from "../components/V2DualPathBar";
import V2ErrorBanner from "../components/V2ErrorBanner";
import {
  canonicalizeJson,
  diffStructuredValues,
  parseStructuredDocument,
} from "../utils/structuredDiff";
import type { StructuredDiffKind } from "../utils/structuredDiff";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

export interface V2StructuredCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

const SAMPLE_LEFT = `{
  "service": "autocompare-api",
  "version": "0.5.0",
  "database": {
    "host": "staging.db.internal",
    "port": 5432,
    "pool": 10
  },
  "features": ["text", "folder", "hex"]
}`;

const SAMPLE_RIGHT = `{
  "version": "0.5.0",
  "service": "autocompare-api",
  "database": {
    "pool": 25,
    "host": "prod.db.internal",
    "port": 5432,
    "ssl": true
  },
  "features": ["hex", "text", "folder", "structured"]
}`;

function badgeForKind(kind: StructuredDiffKind): { label: string; bg: string; fg: string } {
  switch (kind) {
    case "added":
      return { label: "Added", bg: "var(--v2-ins-bg)", fg: "var(--v2-ins-fg)" };
    case "removed":
      return { label: "Removed", bg: "var(--v2-del-bg)", fg: "var(--v2-del-fg)" };
    case "changed":
      return { label: "Changed", bg: "var(--v2-chg-bg)", fg: "var(--v2-chg-fg)" };
    case "typeChanged":
      return { label: "Type", bg: "var(--v2-del-bg)", fg: "var(--v2-del-fg)" };
    default:
      return { label: "Equal", bg: "var(--v2-seg-track)", fg: "var(--v2-fg-muted)" };
  }
}

export default function V2StructuredCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2StructuredCompareProps): React.JSX.Element {
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [leftText, setLeftText] = useState(SAMPLE_LEFT);
  const [rightText, setRightText] = useState(SAMPLE_RIGHT);
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [ignoreArrayOrder, setIgnoreArrayOrder] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);
  const [canonicalized, setCanonicalized] = useState(false);
  const [pathQuery, setPathQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const loadFromPaths = useCallback(async (l: string, r: string): Promise<void> => {
    if (!l.trim() || !r.trim()) return;
    setError(null);
    try {
      const sum = await textCompare(l.trim(), r.trim());
      const page = await textGetRows(sum.session, 0, Math.min(Math.max(sum.rows, 200), 2000));
      const lLines = page.rows.filter((row) => row.left !== null).map((row) => row.left as string);
      const rLines = page.rows.filter((row) => row.right !== null).map((row) => row.right as string);
      if (lLines.length > 0) setLeftText(lLines.join("\n"));
      if (rLines.length > 0) setRightText(rLines.join("\n"));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void loadFromPaths(leftPath, rightPath);
  }, [leftPath, rightPath, loadFromPaths]);

  const parsedLeft = useMemo(() => parseStructuredDocument(leftText), [leftText]);
  const parsedRight = useMemo(() => parseStructuredDocument(rightText), [rightText]);

  const nodes = useMemo(() => {
    if (!parsedLeft.ok || !parsedRight.ok) return [];
    return diffStructuredValues(parsedLeft.value, parsedRight.value, {
      ignoreArrayOrder,
      ignoreCase,
    });
  }, [parsedLeft, parsedRight, ignoreArrayOrder, ignoreCase]);

  const diffCount = useMemo(
    () => nodes.filter((n) => n.kind !== "equal").length,
    [nodes],
  );

  const visibleNodes = useMemo(() => {
    const q = pathQuery.trim().toLowerCase();
    return nodes.filter((n) => {
      if (filterMode === "diffs" && n.kind === "equal") return false;
      if (filterMode === "same" && n.kind !== "equal") return false;
      if (!q) return true;
      return (
        n.path.toLowerCase().includes(q) ||
        n.leftValue.toLowerCase().includes(q) ||
        n.rightValue.toLowerCase().includes(q)
      );
    });
  }, [nodes, filterMode, pathQuery]);

  const handleCanonicalize = (): void => {
    if (parsedLeft.ok) setLeftText(canonicalizeJson(parsedLeft.value));
    if (parsedRight.ok) setRightText(canonicalizeJson(parsedRight.value));
    if (parsedLeft.ok && parsedRight.ok) setCanonicalized(true);
  };

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      swapSides: () => {
        onPathsChange(rightDraft, leftDraft);
        setLeftText(rightText);
        setRightText(leftText);
      },
      reload: () => void loadFromPaths(leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, onPathsChange, leftDraft, rightDraft, leftText, rightText, loadFromPaths]);

  const structuredMode = useMemo(() => {
    if (!parsedLeft.ok || !parsedRight.ok) return "error";
    return `${parsedLeft.format}:${parsedRight.format}`;
  }, [parsedLeft, parsedRight]);

  const structuredStatus = parsedLeft.ok && parsedRight.ok ? "ok" : "error";

  useEffect(() => {
    const fmt =
      parsedLeft.ok && parsedRight.ok
        ? `${parsedLeft.format.toUpperCase()} ↔ ${parsedRight.format.toUpperCase()}`
        : "Parse error";
    onStatusChange({
      left: `${diffCount} semantic ${diffCount === 1 ? "difference" : "differences"}`,
      center: `${fmt} · Key-order insensitive${ignoreArrayOrder ? " · Array-order insensitive" : ""}`,
      right: `${nodes.length} leaf paths`,
    });
  }, [onStatusChange, diffCount, nodes.length, parsedLeft, parsedRight, ignoreArrayOrder]);

  return (
    <div
      data-testid="v2/structured-view"
      data-structured-mode={structuredMode}
      data-structured-status={structuredStatus}
      data-ignore-array-order={String(ignoreArrayOrder)}
      data-canonicalized={String(canonicalized)}
      className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]"
    >
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left JSON / YAML path (or edit below)…"
        rightPlaceholder="Right JSON / YAML path (or edit below)…"
      />

      {/* Semantic Diff Controls */}
      <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5 text-[11.5px]">
        <div className="flex flex-wrap items-center gap-3">
          <span className="inline-flex items-center gap-1 font-semibold text-[var(--v2-fg)]">
            <Braces size={13} className="text-[var(--v2-accent)]" />
            Structural AST Diff
          </span>
          <label className="flex cursor-pointer items-center gap-1.5">
            <input
              type="checkbox"
              checked={ignoreArrayOrder}
              onChange={(e) => setIgnoreArrayOrder(e.target.checked)}
              className="accent-[var(--v2-accent)]"
            />
            <span>Ignore array order</span>
          </label>
          <label className="flex cursor-pointer items-center gap-1.5">
            <input
              type="checkbox"
              checked={ignoreCase}
              onChange={(e) => setIgnoreCase(e.target.checked)}
              className="accent-[var(--v2-accent)]"
            />
            <span>Ignore string case</span>
          </label>
          <button
            type="button"
            onClick={handleCanonicalize}
            disabled={!parsedLeft.ok || !parsedRight.ok}
            className="v2-btn h-[21px] px-2 text-[11px]"
          >
            <Sparkles size={11} />
            Sort Keys & Format
          </button>
        </div>

        <div className="relative">
          <Search
            size={12}
            className="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
          />
          <input
            type="text"
            value={pathQuery}
            onChange={(e) => setPathQuery(e.target.value)}
            placeholder="Filter JSONPath (e.g. $.database)…"
            className="v2-input v2-mono h-[22px] w-56 text-[11px]"
            style={{ paddingLeft: 24 }}
          />
        </div>
      </div>

      <V2ErrorBanner
        message={
          error ??
          (!parsedLeft.ok
            ? `Left parse error: ${parsedLeft.error}`
            : !parsedRight.ok
              ? `Right parse error: ${parsedRight.error}`
              : null)
        }
        onDismiss={() => setError(null)}
      />

      {/* Top Half: Semantic AST Diff Table */}
      <div className="min-h-0 flex-1 overflow-y-auto border-b border-[var(--v2-border)] text-[12px]">
        <div className="sticky top-0 grid grid-cols-[1.2fr_90px_1fr_1fr] border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
          <span>JSONPath</span>
          <span>Status</span>
          <span>Left Value</span>
          <span>Right Value</span>
        </div>
        {visibleNodes.length === 0 ? (
          <div className="flex h-32 items-center justify-center text-[var(--v2-fg-muted)]">
            No matching AST paths.
          </div>
        ) : (
          visibleNodes.map((node) => {
            const badge = badgeForKind(node.kind);
            return (
              <div
                key={node.path}
                className="v2-mono grid grid-cols-[1.2fr_90px_1fr_1fr] items-center border-b border-[var(--v2-border-subtle)]/60 px-3 py-1 text-[11.5px] hover:bg-[var(--v2-btn-hover)]/40"
              >
                <span className="truncate pr-2 font-medium text-[var(--v2-fg)]">{node.path}</span>
                <div>
                  <span
                    style={{ backgroundColor: badge.bg, color: badge.fg }}
                    className="inline-flex rounded px-1.5 py-0.5 font-sans text-[10px] font-semibold"
                  >
                    {badge.label}
                  </span>
                </div>
                <span
                  className="truncate pr-2"
                  style={node.kind !== "equal" && node.leftValue ? { color: "var(--v2-del-fg)" } : undefined}
                >
                  {node.leftValue || "—"}
                </span>
                <span
                  className="truncate"
                  style={node.kind !== "equal" && node.rightValue ? { color: "var(--v2-ins-fg)" } : undefined}
                >
                  {node.rightValue || "—"}
                </span>
              </div>
            );
          })
        )}
      </div>

      {/* Bottom Half: Live Editable Side-by-Side JSON / YAML Input */}
      <div className="grid h-[190px] shrink-0 grid-cols-2 divide-x divide-[var(--v2-border)] bg-[var(--v2-surface-subtle)]">
        <div className="flex min-h-0 flex-col">
          <span className="border-b border-[var(--v2-border-subtle)] px-3 py-1 text-[10.5px] font-semibold uppercase text-[var(--v2-fg-muted)]">
            Left Source (Editable JSON / YAML)
          </span>
          <textarea
            value={leftText}
            onChange={(e) => setLeftText(e.target.value)}
            spellCheck={false}
            className="v2-mono min-h-0 flex-1 resize-none border-0 bg-[var(--v2-surface)] p-2.5 text-[11.5px] leading-[17px] text-[var(--v2-fg)] outline-none"
          />
        </div>
        <div className="flex min-h-0 flex-col">
          <span className="border-b border-[var(--v2-border-subtle)] px-3 py-1 text-[10.5px] font-semibold uppercase text-[var(--v2-fg-muted)]">
            Right Source (Editable JSON / YAML)
          </span>
          <textarea
            value={rightText}
            onChange={(e) => setRightText(e.target.value)}
            spellCheck={false}
            className="v2-mono min-h-0 flex-1 resize-none border-0 bg-[var(--v2-surface)] p-2.5 text-[11.5px] leading-[17px] text-[var(--v2-fg)] outline-none"
          />
        </div>
      </div>
    </div>
  );
}
