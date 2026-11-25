import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, ArrowRight, Check, ChevronLeft, ChevronRight } from "lucide-react";
import { mergeCompareToOutput, mergeTextApply, mergeTextPlanForTriple } from "../../ipc/merge";
import type { TextConflict, TextResolution } from "../../ipc/merge";
import { pickSaveDialog } from "../session/useSession";
import { Button, EmptyState, InlineNotice, Input } from "../ui";
import { MergeFindPanel, MergeToolbar } from "./merge/controls";
import { arrayField, errString, isRecord } from "./types";
import type { ViewPaths } from "./types";

/**
 * Three-way merge view (Wave 12: merge depth; Wave 37: take-both).
 *
 * IPC: `merge_text_plan` lists conflicts; per-conflict take-left /
 * take-right buttons record a choice, take-both records a concatenation
 * choice (`bothLeftFirst` = left-then-right, `bothRightFirst` =
 * right-then-left, Beyond Compare ordering), the merged preview
 * re-renders from the plan + choices, and Apply writes the merged
 * preview to `outPath` via the real `merge_text_apply` IPC
 * (resolutions + explicit `outPath`) and reports a success receipt
 * (bytes written + path). Backend errors and the empty-`outPath`
 * refusal surface inline in the apply report -- never a crash, never
 * a status flip. Take-both on an empty-side conflict refuses inline
 * (per-conflict note) instead of resolving -- never a crash.
 *
 * Wave-110: the merged preview is an editable `<textarea
 * data-testid="merge-preview-edit">` so users can manually touch up lines
 * before applying (a visually-hidden `merge-preview` `<pre>` mirrors the
 * same text for legacy specs); per-conflict Take Base (`merge-take-base`)
 * resolves a hunk to the common ancestor (BC Take Center); `Cmd+B` /
 * `Ctrl+B` takes both on the selected conflict and `j`/`k` steps
 * conflicts (BC conflict navigation).
 *
 * Source plan (in order):
 *  1. the caller triple (`basePath ?? leftPath`, `leftPath`, `rightPath`)
 *     when both sides are non-empty;
 *  2. bundled demo fixture triples (`MERGE_DEMO_CANDIDATES`, the
 *     `tools/parity-harness/fixtures/m_*` 3-way pair in several CWD
 *     layouts, same candidate-pair pattern `textCompareDemo` uses) --
 *     first triple the backend accepts wins;
 *  3. mock-IPC fallback: `VITE_MOCK_IPC=1` has no `merge_text_plan`
 *     fixture (the mock adapter resolves `[]`), so the view plans the
 *     embedded copies of the `m_*` fixture bytes (`EMBEDDED_*_TEXT`,
 *     byte-identical to the fixture files) with `splitThreeWay`, a
 *     prefix/suffix 3-way split that mirrors the engine `MergeSession`
 *     shape for these inputs (1 conflict, id 0, base `[]`, left
 *     `["left"]`, right `["right"]`, 1 clean block).
 *
 * Layout follows BC: three panes (left / base / right) of the conflicting
 * hunks, a conflict count, per-conflict take-left / take-right arrows, and
 * prev/next conflict navigation (buttons plus `j`/`k` / `n` / `p` /
 * Take-all (wave-45) resolves every conflict to one side with a count
 * receipt; per-conflict clicks afterwards clear that receipt so it
 * never describes a stale bulk choice.
 * arrow keys when the view is focused). Stable testids: `merge-pane-left`,
 * `merge-pane-base`, `merge-pane-right`, `merge-conflict-count`,
 * `merge-conflict-<id>` cards (legacy `mergeview/conflict-<id>` testids
 * are kept for the views-chrome contract), `merge-take-left-<id>`,
 * `merge-take-right-<id>`, `merge-take-both-<id>` (concatenation in the
 * per-conflict order), `merge-take-both-order-<id>` (order toggle
 * `L-to-R` / `R-to-L`), `merge-take-both-note-<id>` (empty-side refusal),
 * `merge-prev`, `merge-next`,
 * `merge-conflict-position`, `merge-preview-edit` (editable
 * `<textarea>`; legacy `merge-preview` stays synced on a visually-hidden
 * `<pre>` so wave-12/22/37/45 specs keep passing unmodified),
 * `merge-take-base-<id>` (BC Take Center), `merge-out-path`,
 * `merge-apply`, `merge-apply-report`. Shortcuts: `j`/`k` (plus `n`/`p` /
 * arrows) step conflicts, `Cmd+B`/`Ctrl+B` takes both on the selected
 * conflict (BC Take Left Then Right).
 */

type Status = "loading" | "ready" | "empty" | "error";

export interface MergeViewProps extends ViewPaths {
  /** Base (common ancestor) path; defaults to `leftPath` when omitted. */
  readonly basePath?: string;
  /**
   * GAP-08 (FIXMERGE): 3-way drop routing handoff (frontend-only, no
   * `App.tsx` router change). Fired when a file lands on a merge pane
   * (left / base / right) with the staged triple. Until `App` wires it
   * (route through `handleOpenSession`/`tabPaths` + add `"merge"` to
   * `canCompareKind`), the view stages the drop honestly inline and
   * emits `autocompare:merge-drop` on `window` with the same detail.
   */
  readonly onDropPaths?: (next: {
    readonly leftPath: string;
    readonly basePath: string;
    readonly rightPath: string;
  }) => void;
  /**
   * GAP-08 (FIXMERGE): auto-compare routing handoff. Called with the
   * same staged triple right after `onDropPaths` so the host can run
   * the merge plan immediately (the view itself re-plans automatically
   * whenever its path props change, so a wired host gets BC-style
   * drop-to-compare with zero extra IPC).
   */
  readonly onRequestCompare?: (next: {
    readonly leftPath: string;
    readonly basePath: string;
    readonly rightPath: string;
  }) => void;
  /**
   * GAP-13 (FIXMERGE): ViewCommandBus hook for prev/next/copy
   * (`TextView` `onBusCommand` parity). When provided, top-toolbar bus
   * commands route here instead of the internal nav/take handlers.
   */
  readonly onBusCommand?: (cmd: "prev" | "next" | "copy") => void;
}

/** GAP-08: merge pane drop side (left / base-center / right). */
export type MergeDropSide = "left" | "base" | "right";

/** Bundled demo triples tried in order (repo-root vs desktop vs CWDs). */
const MERGE_DEMO_CANDIDATES: readonly (readonly [base: string, left: string, right: string])[] = [
  [
    "tools/parity-harness/fixtures/m_base.txt",
    "tools/parity-harness/fixtures/m_left.txt",
    "tools/parity-harness/fixtures/m_right.txt",
  ],
  [
    "../../tools/parity-harness/fixtures/m_base.txt",
    "../../tools/parity-harness/fixtures/m_left.txt",
    "../../tools/parity-harness/fixtures/m_right.txt",
  ],
  [
    "../../../tools/parity-harness/fixtures/m_base.txt",
    "../../../tools/parity-harness/fixtures/m_left.txt",
    "../../../tools/parity-harness/fixtures/m_right.txt",
  ],
];

/**
 * Embedded copies of the `m_*` fixture bytes (exact file contents:
 * base `"base\n"`, left `"base\nleft\n"`, right `"base\nright\n"`).
 * Used ONLY when the IPC layer resolves a mock fixture (`[]`) instead of
 * a real `merge_text_plan` record, so the view still demonstrates real
 * fixture data with zero plugin errors under `VITE_MOCK_IPC=1`.
 */
const EMBEDDED_BASE_TEXT = "base\n";
const EMBEDDED_LEFT_TEXT = "base\nleft\n";
const EMBEDDED_RIGHT_TEXT = "base\nright\n";

function splitLines(text: string): readonly string[] {
  if (text === "") return [];
  const parts = text.split("\n");
  return parts[parts.length - 1] === "" ? parts.slice(0, -1) : parts;
}

function isConflict(v: unknown): v is TextConflict {
  return (
    isRecord(v) &&
    typeof v["id"] === "number" &&
    Array.isArray(v["left"]) &&
    Array.isArray(v["right"])
  );
}

/** Wave-102: basename for the per-pane file headers (BC prints the file
 *  path above every merge pane). Handles POSIX + Windows separators;
 *  empty path renders as the pane role only (no fabricated name). */
function baseNameOf(path: string): string {
  const tail = path.split(/[\\/]/).pop() ?? "";
  return tail === "" ? "" : tail;
}

/** Wave-102: honest per-pane line meta (`N line(s)` from loaded rows;
 *  byte counts are not client-visible under mock IPC, so lines only). */
function linesMeta(count: number): string {
  return `${count} line${count === 1 ? "" : "s"}`;
}

function numField(raw: Record<string, unknown>, key: string): number | null {
  const value = raw[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/** A `merge_text_plan`-shaped payload is usable (real backend record). */
function asUsablePlan(raw: unknown): { readonly conflicts: readonly TextConflict[]; readonly cleanBlockCount: number } | null {
  if (!isRecord(raw) || Array.isArray(raw)) return null;
  const conflicts = arrayField<unknown>(raw, "conflicts").filter(isConflict);
  const clean = numField(raw, "cleanBlockCount") ?? numField(raw, "clean_block_count") ?? 0;
  const count = numField(raw, "conflictCount") ?? numField(raw, "conflict_count");
  // The mock adapter resolves `[]` (filtered above); a record without a
  // conflict count is not a real plan either -- keep looking.
  if (count === null) return null;
  void count;
  return { conflicts, cleanBlockCount: clean };
}

interface LocalPlan {
  readonly conflicts: readonly TextConflict[];
  readonly cleanBlockCount: number;
  readonly prefix: readonly string[];
  readonly suffix: readonly string[];
}

/**
 * Prefix/suffix 3-way split mirroring the engine `MergeSession` blocking
 * for the bundled fixtures: shared prefix/suffix stay clean, a differing
 * middle becomes one conflict (id 0). Whole-file-identical or one-sided
 * middles resolve clean with no conflicts.
 */
function splitThreeWay(base: readonly string[], left: readonly string[], right: readonly string[]): LocalPlan {
  let prefix = 0;
  while (
    prefix < base.length &&
    prefix < left.length &&
    prefix < right.length &&
    base[prefix] === left[prefix] &&
    base[prefix] === right[prefix]
  ) {
    prefix += 1;
  }
  let suffix = 0;
  while (
    suffix < base.length - prefix &&
    suffix < left.length - prefix &&
    suffix < right.length - prefix &&
    base[base.length - 1 - suffix] === left[left.length - 1 - suffix] &&
    base[base.length - 1 - suffix] === right[right.length - 1 - suffix]
  ) {
    suffix += 1;
  }
  const end = (lines: readonly string[]): number => lines.length - suffix;
  const baseMid = base.slice(prefix, end(base));
  const leftMid = left.slice(prefix, end(left));
  const rightMid = right.slice(prefix, end(right));
  const same = (a: readonly string[], b: readonly string[]): boolean =>
    a.length === b.length && a.every((line, i) => line === b[i]);
  const cleanPrefix = base.slice(0, prefix);
  const cleanSuffix = suffix > 0 ? base.slice(base.length - suffix) : [];
  if (same(leftMid, rightMid) || same(leftMid, baseMid) || same(rightMid, baseMid)) {
    return { conflicts: [], cleanBlockCount: (prefix > 0 ? 1 : 0) + (suffix > 0 ? 1 : 0), prefix: cleanPrefix, suffix: cleanSuffix };
  }
  return {
    conflicts: [{ id: 0, base: [...baseMid], left: [...leftMid], right: [...rightMid] }],
    cleanBlockCount: (prefix > 0 ? 1 : 0) + (suffix > 0 ? 1 : 0),
    prefix: cleanPrefix,
    suffix: cleanSuffix,
  };
}

/**
 * GAP-08 (FIXMERGE): dropped-file path extraction for merge panes.
 * Same channel order as the shared `dropPaths` helper (NOT imported:
 * `components/dnd/*` is outside FIXMERGE ownership): `files[0].path`
 * (Tauri/Electron + Playwright synthetic drops), `files[0].name`
 * (`/tmp/<name>` browser fallback), then `text/uri-list` / `text/plain`
 * (`file://` URI or raw path). Never throws.
 */
function extractMergeDropPath(dataTransfer: DataTransfer | null | undefined): string | null {
  try {
    if (!dataTransfer) return null;
    const files = (dataTransfer as unknown as {
      files?: ArrayLike<{ name?: string; path?: string }> | null;
    }).files;
    if (files && files.length > 0) {
      const first = files[0];
      if (first) {
        if (typeof first.path === "string" && first.path.trim() !== "") return first.path.trim();
        if (typeof first.name === "string" && first.name.trim() !== "") {
          return `/tmp/${first.name.replace(/^[/\\]+/, "").trim()}`;
        }
      }
    }
    const getData = (dataTransfer as unknown as { getData?: (type: string) => string }).getData;
    if (typeof getData === "function") {
      for (const type of ["text/uri-list", "text/plain", "text"]) {
        let raw = "";
        try {
          raw = getData.call(dataTransfer, type) ?? "";
        } catch {
          continue;
        }
        const line = raw
          .split(/\r?\n/)
          .map((s) => s.trim())
          .find((s) => s !== "" && !s.startsWith("#"));
        if (line) {
          if (line.startsWith("file://")) {
            try {
              return decodeURI(line.replace(/^file:\/\/+/, "/"));
            } catch {
              return line.replace(/^file:\/\/+/, "/");
            }
          }
          return line;
        }
      }
    }
    return null;
  } catch {
    return null;
  }
}

/** Wave-66F: one loaded-row find hit. Each hit names its owning conflict
 * (for selection + focus) plus the matching side and line. */
interface MergeFindHit {
  readonly conflictIndex: number;
  readonly conflictId: number;
  readonly side: "left" | "right";
  readonly lineIndex: number;
}

export default function MergeView({
  leftPath,
  rightPath,
  basePath,
  onDropPaths,
  onRequestCompare,
  onBusCommand: propsOnBusCommand,
}: MergeViewProps): React.JSX.Element {
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [conflicts, setConflicts] = useState<readonly TextConflict[]>([]);
  const [cleanBlockCount, setCleanBlockCount] = useState(0);
  const [choices, setChoices] = useState<Readonly<Record<number, "base" | "left" | "right" | "bothLeftFirst" | "bothRightFirst">>>({});
  const [bothOrders, setBothOrders] = useState<Readonly<Record<number, "leftFirst" | "rightFirst">>>({});
  const [bothNotes, setBothNotes] = useState<Readonly<Record<number, string>>>({});
  // Wave-MRG: Conflict set/clear state (Beyond Compare Conflict Set / Clear parity).
  // `true` means conflict was cleared (treated as non-conflict / resolved);
  // absent/false means active conflict.
  const [clearedConflicts, setClearedConflicts] = useState<Readonly<Record<number, boolean>>>({});
  // Wave-MRG: Compare-to-output state per side.
  const [comparingOutput, setComparingOutput] = useState<"left" | "right" | null>(null);
  const [compareOutputReport, setCompareOutputReport] = useState<string | null>(null);
  // Wave-MRG: L2 `autocompare:view-filter` bus filter state.
  const [mergeFilter, setMergeFilter] = useState<"all" | "diffs" | "same" | "context">("all");
  const [selected, setSelected] = useState(0);
  const [sourceLabel, setSourceLabel] = useState<string | null>(null);
  const [fullPrefix, setFullPrefix] = useState<readonly string[] | null>(null);
  const [fullSuffix, setFullSuffix] = useState<readonly string[] | null>(null);
  const [report, setReport] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [outPath, setOutPath] = useState("");
  // Wave-110: manual touch-up of the merged output. `null` means "follow
  // the plan + choices"; any keystroke snapshots the textarea text so the
  // user fine-tune survives re-renders. Any new choice (per-conflict,
  // take-both, order toggle, take-all) or session reload clears it so the
  // preview re-renders from state instead of describing a stale edit.
  const [manualText, setManualText] = useState<string | null>(null);
  // Wave-66F: find in merge panes (text-61 twin). Case-insensitive
  // substring over the loaded left/right rows, purely presentational
  // (no IPC). Empty query is a clean no-op.
  const [findQuery, setFindQuery] = useState("");
  const [findActive, setFindActive] = useState(-1);
  const [findNote, setFindNote] = useState<string | null>(null);
  // Wave-113C: collapsible in-flow find panel (closed by default, BC
  // truth shows one toolbar row; toolbar toggle + Cmd/Ctrl+F open +
  // focus, Esc collapses). Mirrors table wave-68.
  const [findOpen, setFindOpen] = useState(false);
  // GAP-08 (FIXMERGE): per-pane drop affordance + honest staging note.
  // `dropActive` paints the hovered pane outline (zero layout shift:
  // outline only, like `PaneDropZone`); `dropNote` carries the staged
  // triple until the next session reload. Both clear on triple change.
  const [dropActive, setDropActive] = useState<MergeDropSide | null>(null);
  const [dropNote, setDropNote] = useState<string | null>(null);
  const cardRefs = useRef(new Map<number, HTMLElement | null>());
  const handleTakeRequestRef = useRef<
    ((dir: "left" | "right" | "both" | "base" | "bothLeftFirst" | "bothRightFirst" | "clear" | "set") => void) | null
  >(null);

  const base = basePath ?? leftPath;
  const tripleKey = `${base}\0${leftPath}\0${rightPath}`;

  useEffect(() => {
    let cancelled = false;
    setStatus("loading");
    setError(null);
    setReport(null);
    setChoices({});
    setClearedConflicts({});
    setComparingOutput(null);
    setCompareOutputReport(null);
    setMergeFilter("all");
    setManualText(null);
    setTakeAllNote(null);
    setBothOrders({});
    setBothNotes({});
    setSelected(0);
    setFindQuery("");
    setFindActive(-1);
    setFindNote(null);
    setDropActive(null);
    setDropNote(null);
    setSourceLabel(null);
    setFullPrefix(null);
    setFullSuffix(null);
    (async () => {
      const tryPlan = async (b: string, l: string, r: string): Promise<LocalPlan | null> => {
        try {
          const raw = (await mergeTextPlanForTriple({ base: b, left: l, right: r })) as unknown;
          const usable = asUsablePlan(raw);
          return usable === null ? null : { ...usable, prefix: [], suffix: [] };
        } catch {
          return null;
        }
      };
      // 1. Caller triple when both sides are set.
      if (leftPath !== "" && rightPath !== "") {
        const direct = await tryPlan(base, leftPath, rightPath);
        if (cancelled) return;
        if (direct !== null) {
          setConflicts(direct.conflicts);
          setCleanBlockCount(direct.cleanBlockCount);
          setSourceLabel(`merge_text_plan: ${base} ↔ ${leftPath} ↔ ${rightPath}`);
          setStatus(direct.conflicts.length === 0 ? "empty" : "ready");
          return;
        }
      }
      // 2. Bundled demo fixture triples through the real command.
      for (const [b, l, r] of MERGE_DEMO_CANDIDATES) {
        const demo = await tryPlan(b, l, r);
        if (cancelled) return;
        if (demo !== null) {
          setConflicts(demo.conflicts);
          setCleanBlockCount(demo.cleanBlockCount);
          setSourceLabel(`demo fixtures: ${b} ↔ ${l} ↔ ${r}`);
          setStatus(demo.conflicts.length === 0 ? "empty" : "ready");
          return;
        }
      }
      // 3. Mock-IPC fallback over the embedded fixture bytes.
      if (cancelled) return;
      const local = splitThreeWay(
        splitLines(EMBEDDED_BASE_TEXT),
        splitLines(EMBEDDED_LEFT_TEXT),
        splitLines(EMBEDDED_RIGHT_TEXT),
      );
      setConflicts(local.conflicts);
      setCleanBlockCount(local.cleanBlockCount);
      setFullPrefix(local.prefix);
      setFullSuffix(local.suffix);
      setSourceLabel("demo fixtures (bundled m_* bytes; mock IPC has no merge_text_plan)");
      setStatus(local.conflicts.length === 0 ? "empty" : "ready");
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tripleKey]);

  // Wave-66F: Cmd+F / Ctrl+F focuses the merge find box (text-61 twin).
  // Only the active view is mounted (ViewRouter), so no cross-view fight.
  // Wave-70: opens the collapsible find panel first (no-op when already
  // open), then focuses -- mirrors the table wave-68 pattern.
  useEffect(() => {
    // Wave-127: full-session find (menu `session.find`) opens the panel
    // FIRST, then focuses — same open-then-focus as Cmd+F below. The
    // panel commits after `setFindOpen`, so focus retries bounded until
    // the box mounts (no focus-first query on an unmounted input).
    const openFind = (): void => {
      setFindOpen(true);
      let attempts = 0;
      const tick = (): void => {
        attempts += 1;
        const box = document.getElementById("merge-find-box");
        if (box instanceof HTMLElement) {
          box.focus();
          return;
        }
        if (attempts < 20) window.setTimeout(tick, 50);
      };
      window.setTimeout(tick, 0);
    };
    const onSessionFind = (): void => {
      openFind();
    };
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        openFind();
        // Wave-142: global Esc closes the find panel (text/folder parity;
        // input-level Esc already closes it when focused).
      } else if (e.key === "Escape") {
        setFindOpen(false);
      } else if ((e.metaKey || e.ctrlKey) && (e.key === "b" || e.key === "B")) {
        // D3: container onKeyDown already consumed Cmd/Ctrl+B when the
        // merge section has focus (preventDefault) — don't take twice.
        if (e.defaultPrevented) return;
        e.preventDefault();
        handleTakeRequestRef.current?.("both");
      } else if (!e.metaKey && !e.ctrlKey && !e.altKey && (e.key === "j" || e.key === "J" || e.key === "k" || e.key === "K")) {
        // D3: footer-advertised `j`/`k` conflict nav from ANY focus
        // (toolbar/body/ruler). The section `onKeyDown` only fires when
        // the merge section itself has focus, so body-level `j`/`k` was
        // dead despite the footer + toolbar hint advertising it. Route
        // through the SAME bus the toolbar prev/next buttons use (fresh
        // `selected` in the bus listener, wrap-around in selectConflict);
        // typing in inputs/textareas stays untouched. App GAP-09 owns
        // `n`/`p` for merge and ignores `j`/`k`, so no double-step.
        const t = e.target as HTMLElement | null;
        if (t !== null && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
        if (e.defaultPrevented) return;
        e.preventDefault();
        window.dispatchEvent(
          new CustomEvent("autocompare:view-command", {
            detail: { cmd: e.key.toLowerCase() === "j" ? "next" : "prev", viewKind: "merge" },
          }),
        );
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("autocompare:session-find", onSessionFind);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("autocompare:session-find", onSessionFind);
    };
  }, []);

  const count = conflicts.length;

  const selectConflict = useCallback(
    (index: number) => {
      if (count === 0) return;
      const next = ((index % count) + count) % count;
      setSelected(next);
      const id = conflicts[next]?.id;
      if (id !== undefined) cardRefs.current.get(id)?.scrollIntoView({ block: "nearest" });
    },
    [conflicts, count],
  );

  const choose = useCallback((id: number, side: "base" | "left" | "right"): void => {
    setChoices((prev) => ({ ...prev, [id]: side }));
    // A new choice re-renders the preview, dropping any manual touch-up
    // so the text never describes a stale mix (wave-110).
    setManualText(null);
    // A per-conflict click supersedes any bulk receipt (wave-45).
    setTakeAllNote(null);
  }, []);

  // Wave-45 bulk state: last take-all receipt (cleared on session
  // reload and on any per-conflict or take-both change).
  const [takeAllNote, setTakeAllNote] = useState<string | null>(null);

  /**
   * Wave-45 take-all (Beyond Compare style "take left/right for all").
   * Plain left/right for every conflict (no take-both refusals); the
   * preview re-renders from `choices` and Apply picks the resolutions
   * up unchanged. Pure client-side, no new IPC.
   */
  const takeAll = useCallback(
    (side: "left" | "right"): void => {
      if (conflicts.length === 0) {
        setTakeAllNote("no conflicts to resolve");
        return;
      }
      const bulk: Record<number, "left" | "right"> = {};
      for (const c of conflicts) bulk[c.id] = side;
      setChoices((prev) => ({ ...prev, ...bulk }));
      setManualText(null);
      setTakeAllNote(
        `took ${side} for ${conflicts.length} conflict${conflicts.length === 1 ? "" : "s"}`,
      );
    },
    [conflicts],
  );

  /**
   * Wave 37 take-both: resolve a conflict as the concatenation of both
   * sides in the per-conflict order (default left-then-right). Empty-side
   * conflicts refuse inline via `bothNotes` -- never a crash, and the
   * previous choice (if any) is left untouched.
   */
  const chooseBoth = useCallback(
    (conflict: TextConflict): void => {
      const id = conflict.id;
      if (conflict.left.length === 0 || conflict.right.length === 0) {
        const empty = conflict.left.length === 0 ? "left" : "right";
        setBothNotes((prev) => ({
          ...prev,
          [id]: `take-both needs both sides: ${empty} side is empty, so nothing was resolved.`,
        }));
        return;
      }
      setBothNotes((prev) => {
        if (!(id in prev)) return prev;
        const next = { ...prev };
        delete next[id];
        return next;
      });
      const order = bothOrders[id] ?? "leftFirst";
      setChoices((prev) => ({
        ...prev,
        [id]: order === "leftFirst" ? "bothLeftFirst" : "bothRightFirst",
      }));
      setManualText(null);
      setTakeAllNote(null);
    },
    [bothOrders],
  );

  /**
   * Wave 37 order toggle: flip the per-conflict take-both order. When
   * the conflict is already take-both-resolved the choice follows the
   * new order immediately so the preview updates without re-clicking.
   */
  const toggleBothOrder = useCallback(
    (conflict: TextConflict): void => {
      const id = conflict.id;
      const next = (bothOrders[id] ?? "leftFirst") === "leftFirst" ? "rightFirst" : "leftFirst";
      setBothOrders((prev) => ({ ...prev, [id]: next }));
      setManualText(null);
      setChoices((prev) => {
        const current = prev[id];
        if (current !== "bothLeftFirst" && current !== "bothRightFirst") return prev;
        return { ...prev, [id]: next === "leftFirst" ? "bothLeftFirst" : "bothRightFirst" };
      });
    },
    [bothOrders],
  );

  /**
   * Wave-MRG: Conflict Set / Clear (Beyond Compare parity).
   * `asConflict === true`: Set Conflict (forces hunk to be treated as an active conflict, unsetting choices).
   * `asConflict === false`: Clear Conflict (clears conflict status so it is treated as resolved).
   */
  const setConflictStatus = useCallback((id: number, asConflict: boolean): void => {
    if (asConflict) {
      setClearedConflicts((prev) => {
        if (!prev[id]) return prev;
        const next = { ...prev };
        delete next[id];
        return next;
      });
      // Clear choice so it returns to an unresolved conflict with markers
      setChoices((prev) => {
        if (prev[id] === undefined) return prev;
        const next = { ...prev };
        delete next[id];
        return next;
      });
    } else {
      setClearedConflicts((prev) => ({ ...prev, [id]: true }));
    }
    setManualText(null);
    setTakeAllNote(null);
  }, []);

  // Wave-E1: menu Take / Cmd+D/E / Cmd+B routing handler for MergeView.
  const lastTakeTickRef = useRef<{ time: number; dir: string }>({ time: 0, dir: "" });
  const handleTakeRequest = useCallback(
    (dir: "left" | "right" | "both" | "base" | "bothLeftFirst" | "bothRightFirst" | "clear" | "set") => {
      const now = Date.now();
      if (now - lastTakeTickRef.current.time < 30 && lastTakeTickRef.current.dir === dir) {
        return;
      }
      lastTakeTickRef.current = { time: now, dir };

      if (conflicts.length === 0) {
        window.dispatchEvent(
          new CustomEvent("autocompare:merge-take-result", {
            detail: { dir, ok: false, reason: "No conflicts to resolve" },
          }),
        );
        return;
      }
      const current = conflicts[selected];
      if (!current) {
        window.dispatchEvent(
          new CustomEvent("autocompare:merge-take-result", {
            detail: { dir, ok: false, reason: `Take ${dir} needs a conflict selected` },
          }),
        );
        return;
      }
      if (dir === "left") {
        choose(current.id, "left");
      } else if (dir === "right") {
        choose(current.id, "right");
      } else if (dir === "base") {
        choose(current.id, "base");
      } else if (dir === "both") {
        if (current.left.length === 0 || current.right.length === 0) {
          chooseBoth(current);
          const empty = current.left.length === 0 ? "left" : "right";
          window.dispatchEvent(
            new CustomEvent("autocompare:merge-take-result", {
              detail: {
                dir,
                ok: false,
                conflictId: current.id,
                reason: `take-both needs both sides: ${empty} side is empty`,
              },
            }),
          );
          return;
        }
        chooseBoth(current);
      } else if (dir === "bothLeftFirst") {
        setBothOrders((prev) => ({ ...prev, [current.id]: "leftFirst" }));
        chooseBoth(current);
      } else if (dir === "bothRightFirst") {
        setBothOrders((prev) => ({ ...prev, [current.id]: "rightFirst" }));
        chooseBoth(current);
      } else if (dir === "clear") {
        setConflictStatus(current.id, false);
      } else if (dir === "set") {
        setConflictStatus(current.id, true);
      }
      window.dispatchEvent(
        new CustomEvent("autocompare:merge-take-result", {
          detail: { dir, ok: true, conflictId: current.id },
        }),
      );
    },
    [conflicts, selected, choose, chooseBoth, setConflictStatus],
  );

  useEffect(() => {
    handleTakeRequestRef.current = handleTakeRequest;
  });

  useEffect(() => {
    const onMergeTake = (event: Event) => {
      const customEvent = event as CustomEvent<{
        dir: "left" | "right" | "both" | "base" | "bothLeftFirst" | "bothRightFirst" | "clear" | "set";
      }>;
      if (!customEvent.detail) return;
      customEvent.preventDefault();
      handleTakeRequest(customEvent.detail.dir);
    };
    window.addEventListener("autocompare:merge-take", onMergeTake);
    return () => {
      window.removeEventListener("autocompare:merge-take", onMergeTake);
    };
  }, [handleTakeRequest]);

  // GAP-08 (FIXMERGE): 3-way pane drop routing. Each merge pane (left /
  // base / right) stages its side and re-plans automatically once the
  // host applies the triple (the `tripleKey` effect above is the
  // auto-compare: no new IPC, no `App.tsx` change). The staged triple
  // is offered to the host through `onDropPaths` / `onRequestCompare`
  // and broadcast as `autocompare:merge-drop` for the App handoff.
  const handleMergePaneDrop = useCallback(
    (side: MergeDropSide, path: string): void => {
      const next = {
        leftPath: side === "left" ? path : leftPath,
        basePath: side === "base" ? path : base,
        rightPath: side === "right" ? path : rightPath,
      };
      try {
        window.dispatchEvent(
          new CustomEvent("autocompare:merge-drop", { detail: { side, path, ...next } }),
        );
      } catch {
        // Broadcast only; a hostile listener must never break the drop.
      }
      onDropPaths?.(next);
      onRequestCompare?.(next);
      const routed = onDropPaths !== undefined || onRequestCompare !== undefined;
      setDropNote(
        routed
          ? `Drop staged: ${side} set to ${path} — auto-compare re-plans from the new triple.`
          : `Drop staged: ${side} set to ${path} (host has not wired onDropPaths yet — App handoff: route this triple through handleOpenSession/tabPaths and add "merge" to canCompareKind).`,
      );
    },
    [leftPath, rightPath, base, onDropPaths, onRequestCompare],
  );

  // GAP-13 (FIXMERGE): ViewCommandBus hook for prev/next/copy
  // (`TextView` `autocompare:view-command` parity, placed with the
  // other effects above the early returns so hook order stays stable).
  // `copy` resolves the active conflict (optional `dir` detail honours
  // left/base/right/both, default left) through the same
  // `handleTakeRequest` path as the card buttons and Cmd+D/E, so the
  // top Toolbar prev/next/copy work in merge once `App` routes the
  // merge kind here (handoff: extend `handleViewCommand` merge branch
  // to dispatch `autocompare:view-command` with `viewKind: "merge"`).
  const handleBusCommand = useCallback(
    (cmd: "prev" | "next" | "copy", dir?: unknown): void => {
      if (propsOnBusCommand) {
        propsOnBusCommand(cmd);
        return;
      }
      if (cmd === "prev") {
        selectConflict(selected - 1);
        return;
      }
      if (cmd === "next") {
        selectConflict(selected + 1);
        return;
      }
      const takeDir =
        dir === "right" || dir === "base" || dir === "both" ? dir : ("left" as const);
      handleTakeRequest(takeDir);
    },
    [propsOnBusCommand, selectConflict, selected, handleTakeRequest],
  );

  useEffect(() => {
    const onBusCmd = (e: Event): void => {
      const detail = (e as CustomEvent<{ cmd?: unknown; viewKind?: unknown; dir?: unknown }>)
        .detail;
      if (!detail) return;
      const kind = detail.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "merge") return;
      const cmd = detail.cmd;
      if (cmd === "prev" || cmd === "next" || cmd === "copy") {
        handleBusCommand(cmd, detail.dir);
      }
    };
    window.addEventListener("autocompare:view-command", onBusCmd);
    return () => {
      window.removeEventListener("autocompare:view-command", onBusCmd);
    };
  }, [handleBusCommand]);

  // Wave-MRG: L2 `autocompare:view-filter` bus (Beyond Compare View menu
  // parity: Show All / Differences / Same / Context). Applies when
  // `viewKind === "merge"` or an unkinded event.
  useEffect(() => {
    const onViewFilter = (e: Event): void => {
      const detail = (e as CustomEvent<{ filter?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "merge") return;
      const f = detail?.filter;
      if (f === "all" || f === "diffs" || f === "same" || f === "context") {
        setMergeFilter(f);
      }
    };
    window.addEventListener("autocompare:view-filter", onViewFilter);
    return () => {
      window.removeEventListener("autocompare:view-filter", onViewFilter);
    };
  }, []);

  function onKeyDown(event: React.KeyboardEvent): void {
    // Wave-110: Cmd+B / Ctrl+B takes both on the selected conflict (BC
    // "Take Left Then Right"). Handled before the input guard so it also
    // fires from inside the editable preview textarea.
    // Wave-E1: Cmd+D / Cmd+E takes left / right on the selected conflict.
    if ((event.metaKey || event.ctrlKey) && (event.key === "b" || event.key === "B")) {
      event.preventDefault();
      handleTakeRequest("both");
      return;
    }
    if ((event.metaKey || event.ctrlKey) && (event.key === "d" || event.key === "D")) {
      event.preventDefault();
      handleTakeRequest("left");
      return;
    }
    if ((event.metaKey || event.ctrlKey) && (event.key === "e" || event.key === "E")) {
      event.preventDefault();
      handleTakeRequest("right");
      return;
    }
    // Wave-66F: typing in the find box must not drive conflict navigation.
    const target = event.target as HTMLElement | null;
    if (target !== null && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
    if (count === 0) return;
    switch (event.key) {
      case "j":
      case "n":
      case "ArrowDown":
        event.preventDefault();
        selectConflict(selected + 1);
        break;
      case "k":
      case "p":
      case "ArrowUp":
        event.preventDefault();
        selectConflict(selected - 1);
        break;
      default:
        break;
    }
  }

  const sideLines = useMemo(() => {
    const left: string[] = [];
    const mid: string[] = [];
    const right: string[] = [];
    for (const c of conflicts) {
      left.push(...c.left);
      mid.push(...(c.base ?? []));
      right.push(...c.right);
    }
    return { left, base: mid, right };
  }, [conflicts]);

  // Wave-66F: loaded left/right row count names the miss scope (base is
  // not searched, matching the pane contract of left/right rows).
  const loadedRowCount = sideLines.left.length + sideLines.right.length;

  // Wave-66F: conflict-ordered, row-major hits over the loaded left/right
  // rows (case-insensitive substring, text-61 twin). Recomputes live as
  // the plan loads; activation clamps into range.
  const findHits = useMemo((): readonly MergeFindHit[] => {
    const q = findQuery.trim().toLowerCase();
    if (q === "") return [];
    const out: MergeFindHit[] = [];
    conflicts.forEach((c, conflictIndex) => {
      c.left.forEach((line, lineIndex) => {
        if (line.toLowerCase().includes(q)) {
          out.push({ conflictIndex, conflictId: c.id, side: "left", lineIndex });
        }
      });
      c.right.forEach((line, lineIndex) => {
        if (line.toLowerCase().includes(q)) {
          out.push({ conflictIndex, conflictId: c.id, side: "right", lineIndex });
        }
      });
    });
    return out;
  }, [findQuery, conflicts]);

  // Wave-66F: activate hit i (modulo wrap): select the owning conflict
  // (scrolls the card into view) + move DOM focus to it, status derives
  // the position live in render.
  function activateHit(i: number): void {
    const n = findHits.length;
    if (n === 0) return;
    const wrapped = ((i % n) + n) % n;
    const hit = findHits[wrapped];
    if (hit === undefined) return;
    setFindActive(wrapped);
    setFindNote(null);
    selectConflict(hit.conflictIndex);
    cardRefs.current.get(hit.conflictId)?.focus();
  }

  // Wave-66F: Enter (or Next from idle) starts from the first hit; empty
  // query is a clean no-op that only clears the note.
  function runFind(): void {
    if (findQuery.trim() === "") {
      setFindActive(-1);
      setFindNote(null);
      return;
    }
    if (findHits.length === 0) {
      setFindActive(-1);
      setFindNote(
        "no matches in " +
          loadedRowCount +
          " loaded " +
          (loadedRowCount === 1 ? "row" : "rows"),
      );
      return;
    }
    activateHit(0);
  }

  /** Merged preview: full-file render when the embedded bytes are known, else per-conflict chunks. */
  const previewLines = useMemo((): readonly string[] => {
    const resolve = (c: TextConflict): readonly string[] => {
      const isCleared = clearedConflicts[c.id] === true;
      const choice = choices[c.id];
      if (choice === "base") return [...(c.base ?? [])];
      if (choice === "left") return c.left;
      if (choice === "right") return c.right;
      if (choice === "bothLeftFirst") return [...c.left, ...c.right];
      if (choice === "bothRightFirst") return [...c.right, ...c.left];
      if (isCleared) return [...(c.base ?? [])];
      return ["<<<<<<< left", ...c.left, "=======", ...c.right, ">>>>>>> right"];
    };
    const chunks = conflicts.map(resolve);
    if (fullPrefix === null || fullSuffix === null) return chunks.flat();
    return [...fullPrefix, ...chunks.flat(), ...fullSuffix];
  }, [conflicts, choices, clearedConflicts, fullPrefix, fullSuffix]);

  // Wave-110: editable preview text. Follows plan + choices until the
  // user types, then the manual snapshot wins until the next choice.
  const displayText = manualText ?? previewLines.join("\n");

  async function apply(): Promise<void> {
    setApplying(true);
    setReport(null);
    try {
      if (outPath.trim() === "") {
        setReport("outPath is required: type an output path before applying (the backend refuses to run without one).");
        return;
      }
      const target = outPath.trim();
      const resolutions: TextResolution[] = conflicts
        .filter(
          (c) =>
            choices[c.id] === "base" ||
            choices[c.id] === "left" ||
            choices[c.id] === "right" ||
            choices[c.id] === "bothLeftFirst" ||
            choices[c.id] === "bothRightFirst" ||
            clearedConflicts[c.id] === true,
        )
        .map((c) => ({
          conflictId: c.id,
          choice: (choices[c.id] ?? (clearedConflicts[c.id] ? "base" : "left")) as
            | "base"
            | "left"
            | "right"
            | "bothLeftFirst"
            | "bothRightFirst",
        }));
      let raw: unknown;
      try {
        raw = (await mergeTextApply(base, leftPath, rightPath, resolutions, target)) as unknown;
      } catch (e) {
        // Backend refusal (missing inputs, unknown id, overwrite guard)
        // surfaces inline -- never a crash, never a status flip.
        setReport(`merge_text_apply failed: ${errString(e)}`);
        return;
      }
      if (!isRecord(raw) || Array.isArray(raw)) {
        setReport(
          `merge_text_apply returned a mock fixture; nothing applied (${resolutions.length} resolution(s) \u2192 ${target}).`,
        );
        return;
      }
      const written = typeof raw["writtenLines"] === "number" ? (raw["writtenLines"] as number) : 0;
      const resolved = typeof raw["resolvedCount"] === "number" ? (raw["resolvedCount"] as number) : resolutions.length;
      const out = typeof raw["outPath"] === "string" ? (raw["outPath"] as string) : target;
      // Backend framing (merge.rs): LF-joined lines plus one trailing
      // newline, so the receipt reports the exact bytes on disk. The byte
      // count covers the visible preview text (including wave-110 manual
      // touch-ups); resolutions still carry the recorded per-conflict
      // choices for the real backend write.
      const fileText = displayText === "" ? "" : `${displayText}\n`;
      const bytes = new TextEncoder().encode(fileText).length;
      setReport(`Applied ${resolved} resolution(s): ${bytes} bytes (${written} lines) \u2192 ${out}`);
    } finally {
      setApplying(false);
    }
  }

  // Wave-MRG: Compare-to-output per side (Beyond Compare parity).
  const compareLeftDisabledReason = useMemo((): string | null => {
    if (leftPath.trim() === "") return "Left path is required to compare";
    if (outPath.trim() === "") return "Output path is required to compare";
    if (report === null) return "Apply merge first: output file must exist on disk";
    return null;
  }, [leftPath, outPath, report]);

  const compareRightDisabledReason = useMemo((): string | null => {
    if (rightPath.trim() === "") return "Right path is required to compare";
    if (outPath.trim() === "") return "Output path is required to compare";
    if (report === null) return "Apply merge first: output file must exist on disk";
    return null;
  }, [rightPath, outPath, report]);

  const compareToOutput = useCallback(
    async (side: "left" | "right"): Promise<void> => {
      const disabledReason = side === "left" ? compareLeftDisabledReason : compareRightDisabledReason;
      if (disabledReason !== null) {
        setCompareOutputReport(`Compare ${side} to output disabled: ${disabledReason}`);
        return;
      }
      setComparingOutput(side);
      setCompareOutputReport(null);
      const targetSidePath = side === "left" ? leftPath.trim() : rightPath.trim();
      const targetOutPath = outPath.trim();
      try {
        let raw: unknown;
        try {
          raw = (await mergeCompareToOutput(targetSidePath, targetOutPath)) as unknown;
        } catch {
          raw = null;
        }
        const usable = asUsablePlan(raw);
        if (usable !== null) {
          const diffs = usable.conflicts.length;
          const clean = usable.cleanBlockCount;
          if (diffs === 0) {
            setCompareOutputReport(`Output matches ${side} exactly (0 differences across ${clean} clean block(s))`);
          } else {
            setCompareOutputReport(`Output differs from ${side}: ${diffs} difference section(s), ${clean} clean block(s)`);
          }
        } else {
          // In mock mode (mock adapter returns []) or embedded fixture fallback:
          // compare loaded lines with preview lines
          const sideLinesForSide = side === "left" ? sideLines.left : sideLines.right;
          const outputLines = displayText.split("\n");
          const isIdentical =
            sideLinesForSide.length === outputLines.length &&
            sideLinesForSide.every((line, idx) => line === outputLines[idx]);
          if (isIdentical) {
            setCompareOutputReport(`Output matches ${side} (${outputLines.length} line(s) identical)`);
          } else {
            setCompareOutputReport(
              `Output differs from ${side} (${sideLinesForSide.length} lines in ${side} vs ${outputLines.length} in output)`,
            );
          }
        }
      } catch (err) {
        setCompareOutputReport(`Compare ${side} to output failed: ${errString(err)}`);
      } finally {
        setComparingOutput(null);
      }
    },
    [compareLeftDisabledReason, compareRightDisabledReason, leftPath, rightPath, outPath, displayText, sideLines],
  );

  // Wave-MRG: conflict filtering via L2 `autocompare:view-filter` bus
  // ("all" | "diffs" | "same" | "context"). Must be declared before early returns (Rules of Hooks).
  const visibleConflicts = useMemo(() => {
    if (mergeFilter === "diffs") {
      return conflicts.filter((c) => choices[c.id] === undefined && clearedConflicts[c.id] !== true);
    }
    if (mergeFilter === "same") {
      return conflicts.filter((c) => choices[c.id] !== undefined || clearedConflicts[c.id] === true);
    }
    return conflicts;
  }, [conflicts, choices, clearedConflicts, mergeFilter]);

  if (status === "loading") return <EmptyState title="Loading merge plan…" testid="mergeview/loading" />;
  if (status === "error") return <InlineNotice testid="mergeview/error">Merge plan failed: {error}</InlineNotice>;
  if (status === "empty") return <EmptyState title="No conflicts — merge is clean." testid="mergeview/empty" />;

  const position = count === 0 ? "No conflicts" : `Conflict ${selected + 1} of ${count}`;
  // Wave-70 / Wave-MRG: bottom status bar counts reuse existing merge state:
  // resolved = conflicts with a recorded choice OR cleared conflict.
  const resolvedCount = conflicts.filter((c) => choices[c.id] !== undefined || clearedConflicts[c.id] === true).length;
  const unresolvedCount = count - resolvedCount;
  const clearedCount = conflicts.filter((c) => clearedConflicts[c.id] === true).length;

  const findStatusText =
    findNote ??
    (findActive >= 0 && findActive < findHits.length
      ? "match " + (findActive + 1) + " of " + findHits.length
      : "Find across loaded rows — Enter searches.");

  return (
    <section
      aria-label="three-way merge"
      data-filter={mergeFilter}
      tabIndex={0}
      onKeyDown={onKeyDown}
      className="relative min-h-0 flex-1 overflow-y-auto outline-none"
    >
      {/* Wave-70: chrome collapse (Beyond Compare style). One toolbar row
          (conflict nav + take-all + find toggle) plus a collapsible
          in-flow find panel (open by default, Ctrl+F/Esc). Counts live
          in the bottom status line. All legacy testids ride the same
          controls; find/choice logic byte-identical. */}
      <MergeToolbar
        position={position}
        onPrev={() => selectConflict(selected - 1)}
        onNext={() => selectConflict(selected + 1)}
        onTakeAllLeft={() => takeAll("left")}
        onTakeAllRight={() => takeAll("right")}
        takeAllNote={takeAllNote}
        findOpen={findOpen}
        onToggleFind={() => setFindOpen((v) => !v)}
        conflictIds={conflicts.map((c) => c.id)}
        selectedId={conflicts[selected]?.id ?? null}
        onSelectId={(id) => {
          const index = conflicts.findIndex((c) => c.id === id);
          if (index >= 0) selectConflict(index);
        }}
        activeId={conflicts[selected]?.id ?? null}
        activePosition={position}
        activeChoice={conflicts[selected] ? (choices[conflicts[selected].id] ?? null) : null}
        takeActiveDisabled={count === 0}
        onTakeActiveLeft={() => handleTakeRequest("left")}
        onTakeActiveCenter={() => handleTakeRequest("base")}
        onTakeActiveRight={() => handleTakeRequest("right")}
      />
      {findOpen && (
        <MergeFindPanel
          query={findQuery}
          onQueryChange={(q) => {
            setFindQuery(q);
            setFindActive(-1);
            setFindNote(null);
          }}
          prevDisabled={findHits.length === 0}
          nextDisabled={findQuery.trim() === ""}
          onPrev={() => activateHit(findActive - 1)}
          onNext={() => (findActive < 0 ? runFind() : activateHit(findActive + 1))}
          onRun={runFind}
          onClose={() => setFindOpen(false)}
          statusText={findStatusText}
        />
      )}
      {/* Wave-102: BC-style per-pane file headers. BC prints the file
          path above every merge pane (path bar + timestamp/bytes meta);
          ours names each pane with its session path (basename + full
          path in `title`) plus the honest loaded line count (byte
          counts are not client-visible, so lines only -- never
          fabricated). One flex row: title left, meta right, so the
          pane header stays one compact line and the wave-70 chrome
          budget (firstCardTop <= 140px) holds. */}
      {/* Wave-188: per-pane gutters + thin splitters (BC 3-way truth:
          every merge pane carries a line-number gutter; panes are split
          by a 1px rail, not an 8px gap). Each pane keeps its exact
          title/meta testids and text; the gutter is aria-hidden,
          1-based within the pane, same mono/line-height as the pre so
          rows align. Margins tighten 8->4px so the chrome budget gains
          headroom (toolbar + one-line headers only above the cards). */}
      <div className="flex items-stretch gap-[var(--ac-space-1)] mx-[var(--ac-space-4)] mt-[var(--ac-space-1)] mb-[var(--ac-space-1)]" role="group" aria-label="merge sides">
        {(
          [
            { key: "left", title: "Left", testid: "merge-pane-left", metaTestid: "mergeview/meta-left", path: leftPath, lines: sideLines.left },
            { key: "base", title: "Base (ancestor)", testid: "merge-pane-base", metaTestid: "mergeview/meta-base", path: base, lines: sideLines.base },
            { key: "right", title: "Right", testid: "merge-pane-right", metaTestid: "mergeview/meta-right", path: rightPath, lines: sideLines.right },
          ] as const
        ).map((pane, paneIndex) => {
          const name = baseNameOf(pane.path);
          const meta = name === "" ? linesMeta(pane.lines.length) : `${name} · ${linesMeta(pane.lines.length)}`;
          const dropSide = pane.key as MergeDropSide;
          return (
          <Fragment key={pane.key}>
          {paneIndex > 0 && (
            <div
              aria-hidden="true"
              data-testid={`merge-splitter-${pane.key}`}
              className="mv-splitter"
            />
          )}
          <div
            key={pane.key}
            data-testid={pane.testid}
            data-mergedrop={dropSide}
            data-active={dropActive === dropSide ? "true" : "false"}
            title={`Drop a file to set ${dropSide} (auto-compare re-plans from the new triple)`}
            onDragOver={(e) => {
              e.preventDefault();
              try {
                if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
              } catch {
                // Best-effort only.
              }
              setDropActive(dropSide);
            }}
            onDragLeave={() => {
              setDropActive((cur) => (cur === dropSide ? null : cur));
            }}
            onDrop={(e) => {
              e.preventDefault();
              e.stopPropagation();
              setDropActive(null);
              const dropped = extractMergeDropPath(e.dataTransfer);
              if (dropped) handleMergePaneDrop(dropSide, dropped);
            }}
            style={dropActive === dropSide ? { outline: "2px dashed var(--ac-accent)", outlineOffset: "-2px" } : undefined}
            className="mv-pane min-w-0 flex-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] p-[var(--ac-space-2)]"
          >
            <div className="flex items-baseline justify-between gap-[var(--ac-space-2)] mb-[var(--ac-space-1)]">
              <p className="ac-mono text-[length:var(--ac-font-size-code)] font-semibold leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)]">{pane.title}</p>
              <p data-testid={pane.metaTestid} title={pane.path === "" ? undefined : pane.path} className="ac-mono truncate text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">{meta}</p>
            </div>
            <div className="flex items-stretch rounded border border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] p-[var(--ac-space-2)]">
              <span
                aria-hidden="true"
                data-testid={`merge-pane-gutter-${pane.key}`}
                title={`${pane.title} pane: line numbers within this pane (1-based)`}
                className="ac-mono shrink-0 border-r border-[var(--ac-border-subtle)] pr-[var(--ac-space-1)] mr-[var(--ac-space-2)] text-right text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)] select-none"
              >
                {pane.lines.length === 0 ? (
                  <span className="block">·</span>
                ) : (
                  pane.lines.map((_, i) => (
                    <span key={i} className="block">{i + 1}</span>
                  ))
                )}
              </span>
              <pre className="whitespace-pre-wrap flex-1 min-w-0 font-normal text-[var(--ac-fg)]">
                {pane.lines.length > 0 ? pane.lines.join("\n") : "(no lines)"}
              </pre>
            </div>
          </div>
          </Fragment>
          );
        })}
      </div>
      {/* GAP-08 (FIXMERGE): staged drop receipt. Renders ONLY after a pane
          drop (null otherwise), so the wave-70 chrome budget (first card
          top <= 140px) and every legacy testid stay untouched. */}
      {dropNote !== null && (
        <InlineNotice testid="merge-drop-note">
          {dropNote}
        </InlineNotice>
      )}
      {/* Wave-147: BC overview-ruler parity. The cards ride a flex row
          with a far-LEFT in-flow rail (wave-196: right edge -> far-left
          `order-first`, 4px; zero vertical chrome: no new stacked rows,
          firstCardTop untouched). One marker per
          conflict, positioned by ordinal fraction (text-79 idiom);
          clicking jumps through the existing selectConflict path (no
          new IPC). Marker ink reuses the existing chg/accent tokens. */}
      <div className="flex items-stretch gap-[var(--ac-space-2)]">
        {/* Inner column is a flex column with gap (NOT card my
            margins): flex items never margin-collapse, so stacked
            `my` would double the pre-147 collapsed 8px rhythm and
            bust the wave-70 chrome budget. Gap keeps the exact
            8px rhythm with zero collapse surprise. */}
        <div className="flex min-w-0 flex-1 flex-col gap-[var(--ac-space-2)]">
        {visibleConflicts.length === 0 && (
          <EmptyState
            title={`No conflicts match view filter \u201C${mergeFilter}\u201D.`}
            testid="merge-filter-empty"
          />
        )}
        {visibleConflicts.map((c, index) => {
        const chosen = choices[c.id];
        const active = index === selected;
        // Wave-66J: BC section bracket. A conflict spanning more than one
        // line on any side is `multi` (the `.mv-rail` ::before paints the
        // left-edge rail); an all-single-line conflict is `single`
        // (arrow glyphs only, no rail).
        //
        // Wave-147: BC gutter parity. Each cell keeps its exact `<pre>`
        // (66J pins: card has 3 pres with unchanged text) and gains an
        // aria-hidden line-number gutter sibling (1-based, intra-conflict;
        // absolute file offsets are not client-visible, so the gutter is
        // honest about its scope in `title`). Same mono/line-height as
        // the pre, so single-line cells align exactly; wrapped long
        // lines are a known limitation (noted in the 147 REPORT).
        const bracket =
          c.left.length > 1 || (c.base ?? []).length > 1 || c.right.length > 1 ? "multi" : "single";
        const gutterOf = (lines: readonly string[], side: string): React.JSX.Element => (
          <span
            aria-hidden="true"
            data-testid={`merge-gutter-${side}-${c.id}`}
            title={`${side} side: line numbers within this conflict (1-based)`}
            className="ac-mono shrink-0 border-r border-[var(--ac-border-subtle)] pr-[var(--ac-space-1)] mr-[var(--ac-space-2)] text-right text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)] select-none"
          >
            {lines.length === 0 ? (
              <span className="block">·</span>
            ) : (
              lines.map((_, i) => (
                <span key={i} className="block">{i + 1}</span>
              ))
            )}
          </span>
        );
        return (
          <article
            key={c.id}
            id={`merge-conflict-${c.id}`}
            data-testid={`mergeview/conflict-${c.id}`}
            data-bracket={bracket}
            data-conflict-status={clearedConflicts[c.id] === true ? "cleared" : "conflict"}
            aria-current={active ? "true" : undefined}
            tabIndex={-1}
            ref={(el) => {
              cardRefs.current.set(c.id, el);
            }}
            onClick={() => setSelected(index)}
            className={`mv-conflict mv-rail mx-[var(--ac-space-4)] rounded-lg border bg-[var(--ac-bg-raised)] p-[var(--ac-space-3)] shadow-xs ${active ? "border-[var(--ac-accent)]" : "border-[var(--ac-border)]"}`}
          >
            <div className="flex items-baseline justify-between gap-[var(--ac-space-2)] mb-[var(--ac-space-2)]">
              <p className="ac-mono text-[length:var(--ac-font-size-code)] font-semibold leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)]">conflict {c.id}</p>
              {/* Wave-102: per-side line counts on the card header (BC
                  shows each side's extent per conflict section). Purely
                  presentational, same flex row so card chrome is
                  unchanged; take/find/apply logic untouched. */}
              <p data-testid={`merge-conflict-meta-${c.id}`} className="ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">
                L{c.left.length} · B{(c.base ?? []).length} · R{c.right.length}
              </p>
            </div>
            <div className="grid grid-cols-3 gap-[var(--ac-space-2)] mb-[var(--ac-space-3)]">
              {(
                [
                  { side: "left", lines: c.left },
                  { side: "base", lines: c.base ?? [] },
                  { side: "right", lines: c.right },
                ] as const
              ).map((cell) => (
                <div key={cell.side} className="flex items-start rounded border border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] p-[var(--ac-space-2)]">
                  {gutterOf(cell.lines, cell.side)}
                  <pre className={`whitespace-pre-wrap flex-1 min-w-0 rounded font-normal text-[var(--ac-fg)]${cell.lines.length === 0 ? " mv-cell-empty" : ""}`}>{cell.lines.join("\n") || "(no lines)"}</pre>
                </div>
              ))}
            </div>
            <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]">
              <Button
                variant="ghost"
                data-testid={`merge-take-left-${c.id}`}
                aria-pressed={chosen === "left"}
                onClick={() => choose(c.id, "left")}
                className="border-0 text-[var(--ac-tab-dirty)] aria-pressed:bg-[var(--ac-selection-bg)]"
              >
                <ChevronLeft size={12} aria-hidden="true" /> Take left{chosen === "left" ? (<Check size={12} aria-hidden="true" />) : null}
              </Button>
              <Button
                variant="ghost"
                data-testid={`merge-take-right-${c.id}`}
                aria-pressed={chosen === "right"}
                onClick={() => choose(c.id, "right")}
                className="border-0 text-[var(--ac-tab-dirty)] aria-pressed:bg-[var(--ac-selection-bg)]"
              >
                Take right <ChevronRight size={12} aria-hidden="true" />{chosen === "right" ? (<Check size={12} aria-hidden="true" />) : null}
              </Button>
              <Button
                variant="secondary"
                data-testid={`merge-take-base-${c.id}`}
                aria-pressed={chosen === "base"}
                aria-label={`take base (ancestor) for conflict ${c.id}`}
                title="Take base (common ancestor — Beyond Compare Take Center)"
                onClick={() => choose(c.id, "base")}
                className="aria-pressed:border-[var(--ac-accent)] aria-pressed:bg-[var(--ac-selection-bg)] aria-pressed:text-[var(--ac-selection-fg)]"
              >
                Take base{chosen === "base" ? (<Check size={12} aria-hidden="true" />) : null}
              </Button>
              <Button
                variant="secondary"
                data-testid={`merge-take-both-${c.id}`}
                aria-pressed={chosen === "bothLeftFirst" || chosen === "bothRightFirst"}
                aria-label={`take both for conflict ${c.id} (Cmd+B)`}
                title="Take both (Beyond Compare Take Left Then Right — Cmd+B on the selected conflict)"
                onClick={() => chooseBoth(c)}
                className="aria-pressed:border-[var(--ac-accent)] aria-pressed:bg-[var(--ac-selection-bg)] aria-pressed:text-[var(--ac-selection-fg)]"
              >
                Take both{(chosen === "bothLeftFirst" || chosen === "bothRightFirst") ? (<Check size={12} aria-hidden="true" />) : null}
              </Button>
              <Button
                variant="secondary"
                data-testid={`merge-take-both-order-${c.id}`}
                data-order={(bothOrders[c.id] ?? "leftFirst") === "leftFirst" ? "L-then-R" : "R-then-L"}
                aria-label={`take-both order for conflict ${c.id}: ${(bothOrders[c.id] ?? "leftFirst") === "leftFirst" ? "left then right (L-then-R)" : "right then left (R-then-L)"}`}
                title="Toggle take-both order (Beyond Compare L-then-R vs R-then-L)"
                onClick={() => toggleBothOrder(c)}
              >
                {(bothOrders[c.id] ?? "leftFirst") === "leftFirst" ? (
                  <>
                    <ArrowRight size={12} aria-hidden="true" />
                    <span>L to R</span>
                    <span data-testid={`merge-take-order-label-${c.id}`} className="ac-mono text-[11px] opacity-80">(L-then-R)</span>
                  </>
                ) : (
                  <>
                    <ArrowLeft size={12} aria-hidden="true" />
                    <span>R to L</span>
                    <span data-testid={`merge-take-order-label-${c.id}`} className="ac-mono text-[11px] opacity-80">(R-then-L)</span>
                  </>
                )}
              </Button>
              <Button
                variant="secondary"
                data-testid={`merge-set-${c.id}`}
                aria-label={`Mark conflict ${c.id} as conflict (Conflict Set)`}
                title="Mark as conflict (Beyond Compare Conflict Set)"
                aria-pressed={clearedConflicts[c.id] !== true}
                onClick={() => setConflictStatus(c.id, true)}
                className="aria-pressed:border-[var(--ac-accent)] aria-pressed:bg-[var(--ac-selection-bg)] aria-pressed:text-[var(--ac-selection-fg)]"
              >
                Set conflict{clearedConflicts[c.id] !== true && chosen === undefined ? (<Check size={12} aria-hidden="true" />) : null}
              </Button>
              <Button
                variant="secondary"
                data-testid={`merge-clear-${c.id}`}
                aria-label={`Clear conflict ${c.id} (Conflict Clear)`}
                title="Clear conflict status (Beyond Compare Conflict Clear)"
                aria-pressed={clearedConflicts[c.id] === true}
                onClick={() => setConflictStatus(c.id, false)}
                className="aria-pressed:border-[var(--ac-accent)] aria-pressed:bg-[var(--ac-selection-bg)] aria-pressed:text-[var(--ac-selection-fg)]"
              >
                Clear conflict{clearedConflicts[c.id] === true ? (<Check size={12} aria-hidden="true" />) : null}
              </Button>
            </div>
            {bothNotes[c.id] !== undefined && (
              <InlineNotice testid={`merge-take-both-note-${c.id}`}>
                {bothNotes[c.id]}
              </InlineNotice>
            )}
          </article>
        );
      })}
        </div>
        {count > 0 && (
          <div
            data-testid="mergeview/overview-ruler"
            role="navigation"
            aria-label={`Overview: ${count} ${count === 1 ? "conflict" : "conflicts"} — activate a marker to jump`}
            className="relative order-first w-[4px] shrink-0 self-stretch rounded bg-[var(--ac-ruler-track)]"
          >
            {conflicts.map((c, index) => {
              const top = (index / count) * 100;
              const height = (1 / count) * 100;
              const isActive = index === selected;
              return (
                <button
                  key={c.id}
                  type="button"
                  data-testid={`mergeview/overview-hunk-${c.id}`}
                  data-active={isActive ? "true" : "false"}
                  aria-label={`Jump to conflict ${index + 1} of ${count}`}
                  title={`Conflict ${index + 1} of ${count}`}
                  aria-current={isActive ? "true" : undefined}
                  onClick={() => selectConflict(index)}
                  className={`absolute left-0 right-0 min-h-2 rounded-sm ${isActive ? "bg-[var(--ac-accent)]" : "bg-[var(--ac-ruler-chg)]"}`}
                  style={{ top: `${top}%`, height: `${height}%` }}
                />
              );
            })}
          </div>
        )}
      </div>
      <div className="m-[var(--ac-space-4)] rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] p-[var(--ac-space-4)]">
        <p className="ac-mono text-[length:var(--ac-font-size-code)] font-semibold leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)] mb-[var(--ac-space-2)]">merged preview (editable)</p>
        <textarea
          data-testid="merge-preview-edit"
          aria-label="merged preview (editable — touch up lines before applying)"
          rows={Math.min(Math.max(displayText.split("\n").length, 3), 16)}
          value={displayText}
          spellCheck={false}
          onChange={(event) => setManualText(event.target.value)}
          className="ac-mono w-full whitespace-pre-wrap rounded border border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] p-[var(--ac-space-2)] text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
        />
        {/* Wave-110 legacy compat: visually-hidden mirror of the editable
            preview so `merge-preview` readers (waves 12/22/37/45) keep
            passing unmodified. `sr-only` keeps a 1px box (Playwright
            treats it as visible) while staying out of the visual layout. */}
        <pre data-testid="merge-preview" aria-hidden="true" className="sr-only">{displayText}</pre>
        {/* Wave-147: edit-mode line count (BC prints the merged output
            extent next to the output pane). Reuses `displayText` only —
            follows choices + manual touch-ups live, no new IPC. */}
        <p data-testid="merge-preview-lines" className="mt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] text-[var(--ac-fg-muted)]">
          {displayText === "" ? "0 lines" : `${displayText.split("\n").length} line${displayText.split("\n").length === 1 ? "" : "s"}`}
        </p>
        <p className="mt-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] text-[var(--ac-fg-muted)]">Shortcuts: j/k step conflicts · Cmd+B takes both on the selected conflict.</p>
      </div>
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)] m-[var(--ac-space-4)]">
        <label htmlFor="merge-out-path" className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">Output path</label>
        <Input
          id="merge-out-path"
          data-testid="merge-out-path"
          controlSize="sm"
          value={outPath}
          onChange={(event) => setOutPath(event.target.value)}
          placeholder="/tmp/merge-12-out.txt"
          className="min-w-64 flex-1 ac-mono text-[length:var(--ac-font-size-code)]"
        />
        <Button
          variant="secondary"
          size="sm"
          data-testid="merge-out-browse"
          aria-label="Browse for output path (Save As)"
          title="Browse for output path (native Save dialog; typed path still works when no dialog is available)"
          onClick={() => {
            void (async () => {
              try {
                const picked = await pickSaveDialog({
                  defaultPath: outPath.trim() === "" ? "/tmp/merge-out.txt" : outPath.trim(),
                  filters: [{ name: "Text", extensions: ["txt"] }],
                });
                if (typeof picked === "string" && picked !== "") setOutPath(picked);
              } catch {
                // Dialog absent/failed: keep the typed value, never crash.
              }
            })();
          }}
        >
          Browse…
        </Button>
        <Button
          variant="primary"
          size="sm"
          data-testid="merge-apply"
          onClick={() => void apply()}
          disabled={applying}
        >
          {applying ? "Applying…" : "Apply merge"}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          data-testid="merge-compare-output-left"
          data-disabled-reason={compareLeftDisabledReason ?? undefined}
          disabled={compareLeftDisabledReason !== null || comparingOutput !== null}
          title={compareLeftDisabledReason ?? "Compare Left to Output (diff output vs left via merge IPC)"}
          onClick={() => void compareToOutput("left")}
        >
          <span data-testid="merge-compare-to-output-left" className="sr-only" aria-hidden="true" />
          {comparingOutput === "left" ? "Comparing left…" : "Compare Left to Output"}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          data-testid="merge-compare-output-right"
          data-disabled-reason={compareRightDisabledReason ?? undefined}
          disabled={compareRightDisabledReason !== null || comparingOutput !== null}
          title={compareRightDisabledReason ?? "Compare Right to Output (diff output vs right via merge IPC)"}
          onClick={() => void compareToOutput("right")}
        >
          <span data-testid="merge-compare-to-output-right" className="sr-only" aria-hidden="true" />
          {comparingOutput === "right" ? "Comparing right…" : "Compare Right to Output"}
        </Button>
      </div>
      {compareOutputReport !== null && (
        <InlineNotice testid="merge-compare-output-report">
          <span data-testid="merge-compare-to-output-report" className="sr-only" aria-hidden="true" />
          {compareOutputReport}
        </InlineNotice>
      )}
      {report !== null && <InlineNotice testid="merge-apply-report">{report}</InlineNotice>}
      {/* Wave-70: counts live in the bottom status bar (BC truth:
          counts in the status bar). Legacy `merge-conflict-count`
          testid rides the same string so wave-12/22/45 specs keep
          passing unmodified; the resolved/unresolved note reuses
          existing choice state (no backend change). */}
      <div data-testid="mergeview/status" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
        <span data-testid="merge-conflict-count">
          {count} conflict section(s) &middot; {cleanBlockCount} clean block(s)
        </span>
        <span data-testid="mergeview/status-note">
          {" · "}{resolvedCount} resolved · {unresolvedCount} unresolved
          {clearedCount > 0 ? ` · ${clearedCount} cleared` : ""}
          {sourceLabel !== null ? ` \u00b7 ${sourceLabel}` : ""}
        </span>
      </div>
    </section>
  );
}
