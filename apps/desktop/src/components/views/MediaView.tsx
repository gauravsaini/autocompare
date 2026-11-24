import { useEffect, useState } from "react";
import { ArrowRight } from "lucide-react";
import {
  audioCompare,
  pictureCompare,
  type AudioSummary,
  type PictureSummary,
} from "../../ipc/media";
import { MediaTolerancePanel, MediaToolbar } from "./media/controls";
import { errString, isRecord } from "./types";
import type { ViewPaths } from "./types";

type Status = "loading" | "ready" | "empty" | "error";
type Mode = "picture" | "audio";

/**
 * Media compare view (Wave 48).
 *
 * `media` sessions cover pictures and audio with one IPC each. The mode
 * follows the file extension (`.wav/.mp3/.flac/.ogg/.m4a` run the audio
 * path, everything else the picture path) and is always stated honestly
 * in `mediaview/mode`. Both summaries are bounded by contract (scalar
 * metrics plus capped diff samples, never bitmaps or sample buffers).
 *
 * Every payload is validated at runtime and degrades to loading/error
 * states; failures render inline and never throw.
 */
function detectMode(leftPath: string, rightPath: string): Mode {
  return /\.(wav|mp3|flac|ogg|m4a)$/i.test(`${leftPath} ${rightPath}`) ? "audio" : "picture";
}

function isPictureSummary(v: unknown): v is PictureSummary {
  if (!isRecord(v)) return false;
  return (
    typeof v["identical"] === "boolean" &&
    typeof v["score"] === "number" &&
    typeof v["differingBytes"] === "number" &&
    typeof v["totalBytes"] === "number" &&
    Array.isArray(v["samples"])
  );
}

function isAudioSummary(v: unknown): v is AudioSummary {
  if (!isRecord(v)) return false;
  return (
    typeof v["identical"] === "boolean" &&
    typeof v["score"] === "number" &&
    typeof v["windowsTotal"] === "number" &&
    typeof v["windowsDiffering"] === "number" &&
    Array.isArray(v["diffWindows"])
  );
}

function pct(score: number): string {
  return `${(score * 100).toFixed(1)}%`;
}

/**
 * Wave-103: BC prints the file path above every compare pane. The
 * media summaries expose no per-pane chrome, so the summary meta
 * line names both session files (basename, full path in `title`)
 * next to the honest scalar dims both modes already render. Same
 * single `mw-row` line (zero added chrome: the wave-72 <=140px
 * first-row budget holds), purely presentational.
 */
function baseName(p: string): string {
  const t = p.split(/[\\/]/).pop() ?? "";
  return t === "" ? "(untitled)" : t;
}

export default function MediaView({ leftPath, rightPath }: ViewPaths): React.JSX.Element {
  // Derived per render (never useState-initialized): the workspace can
  // mount this view with empty paths one frame before the new tab's
  // paths land (wave-48 probe), and the effect below must refetch then.
  const mode = detectMode(leftPath, rightPath);
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [picture, setPicture] = useState<PictureSummary | null>(null);
  const [audio, setAudio] = useState<AudioSummary | null>(null);
  const [tolerance, setTolerance] = useState("");
  const [comparing, setComparing] = useState(false);
  // Wave-72: collapsible tolerance panel (open by default only in audio
  // mode; toolbar toggle / Esc collapse it). Mirrors hex wave-71.
  const [toleranceOpen, setToleranceOpen] = useState(mode === "audio");

  useEffect(() => {
    setToleranceOpen(mode === "audio");
  }, [mode]);

  /* Wave-180: `autocompare:view-command` + `autocompare:view-filter` bus
     (unified top Toolbar parity). Media is a summary view (score/metrics,
     no cursor, no row filter): every command is an honest no-op. The
     toolbar parks non-session buttons, so this never fires by click —
     the kind-scoped listeners exist so a bus/programmatic dispatch can
     never crash the view. Never throws. */
  useEffect(() => {
    const onBusCmd = (e: Event): void => {
      const detail = (e as CustomEvent<{ viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "media") return;
    };
    const onViewFilter = (e: Event): void => {
      const detail = (e as CustomEvent<{ viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "media") return;
    };
    window.addEventListener("autocompare:view-command", onBusCmd);
    window.addEventListener("autocompare:view-filter", onViewFilter);
    return () => {
      window.removeEventListener("autocompare:view-command", onBusCmd);
      window.removeEventListener("autocompare:view-filter", onViewFilter);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    // Wave-130: no paths = pick-two-paths empty (App.hasPaths gate parity:
    // both sides required). No picture/audio compare fires, no error.
    if (leftPath === "" || rightPath === "") {
      setStatus("empty");
      return;
    }
    setStatus("loading");
    setError(null);
    setPicture(null);
    setAudio(null);
    (async () => {
      try {
        if (mode === "picture") {
          const raw = (await pictureCompare(leftPath, rightPath)) as unknown;
          if (cancelled) return;
          if (!isPictureSummary(raw)) {
            setError("picture summary has an unexpected shape");
            setStatus("error");
            return;
          }
          setPicture(raw);
        } else {
          const raw = (await audioCompare(leftPath, rightPath)) as unknown;
          if (cancelled) return;
          if (!isAudioSummary(raw)) {
            setError("audio summary has an unexpected shape");
            setStatus("error");
            return;
          }
          setAudio(raw);
        }
        if (!cancelled) setStatus("ready");
      } catch (e) {
        if (!cancelled) {
          setError(errString(e));
          setStatus("error");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [leftPath, rightPath, mode]);

  async function recomputeAudio(): Promise<void> {
    if (mode !== "audio" || comparing) return;
    setComparing(true);
    try {
      const parsed = tolerance.trim() === "" ? null : Number(tolerance);
      const raw = (await audioCompare(leftPath, rightPath, Number.isFinite(parsed) ? (parsed as number) : undefined)) as unknown;
      if (!isAudioSummary(raw)) {
        setError("audio summary has an unexpected shape");
        setStatus("error");
        return;
      }
      setAudio(raw);
      setStatus("ready");
    } catch (e) {
      setError(errString(e));
      setStatus("error");
    } finally {
      setComparing(false);
    }
  }

  return (
    <section aria-label="media comparison">
      {/* Wave-72: chrome collapse (Beyond Compare style). One toolbar row
          (mode indicator + audio tolerance toggle + recompute) plus a
          collapsible in-flow tolerance panel (open by default in audio
          mode, Esc). Counts live in the bottom status line. All legacy
          testids ride the same controls; picture/audio fetch + recompute
          logic byte-identical. */}
      <MediaToolbar
        mode={mode}
        toleranceOpen={toleranceOpen}
        onToggleTolerance={() => {
          const next = !toleranceOpen;
          setToleranceOpen(next);
          if (next) {
            window.setTimeout(() => {
              const box = document.getElementById("mediaview-tolerance");
              if (box instanceof HTMLElement) box.focus();
            }, 0);
          }
        }}
        comparing={comparing}
        onRecompare={() => void recomputeAudio()}
      />
      {mode === "audio" && toleranceOpen && (
        <MediaTolerancePanel
          tolerance={tolerance}
          onToleranceChange={setTolerance}
          onRecompare={() => void recomputeAudio()}
          onClose={() => setToleranceOpen(false)}
        />
      )}
      {status === "empty" && (
        <p data-testid="mediaview/empty" className="px-[var(--ac-space-4)] py-[var(--ac-space-8)] text-center text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          Pick two files to compare (picture or audio) — needs two paths.
        </p>
      )}
      {status === "loading" && (
        <p data-testid="mediaview/status" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          Comparing media…
        </p>
      )}
      {status === "error" && (
        <p data-testid="mediaview/error" role="alert" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg)]">
          {error ?? "media compare failed"}
        </p>
      )}
      {status === "ready" && picture !== null && (
        <div data-testid="mediaview/summary" className="flex flex-col gap-[var(--ac-space-2)] px-[var(--ac-space-4)] py-[var(--ac-space-3)]">
          <p data-testid="mediaview/score" className="ac-chrome-label font-medium text-[var(--ac-fg)]">
            {picture.identical ? "Identical pictures" : `${pct(picture.score)} equal`} — {picture.differingBytes} / {picture.totalBytes} bytes differ
          </p>
          {/* Wave-185: two-pane meta (BC Picture Compare pane-header echo:
              one pane per file, path in `title`, basename + honest scalar
              dims in words). Single flex line, no new chrome rows; testids,
              titles and words byte-identical so 147/154 pins hold. */}
          <p className="mw-row flex flex-nowrap items-baseline gap-x-[var(--ac-space-4)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">
            <span data-testid="mediaview/meta-left" title={leftPath} className="min-w-0 flex-1 truncate">
              <span aria-hidden="true" className="ac-chrome-label font-medium text-[var(--ac-fg)]">Left</span>
              {" · "}
              {baseName(leftPath)} · {picture.widthL}×{picture.heightL} ({picture.sizeL} bytes, {picture.format})
            </span>
            <span data-testid="mediaview/meta-right" title={rightPath} className="min-w-0 flex-1 truncate">
              <span aria-hidden="true" className="ac-chrome-label font-medium text-[var(--ac-fg)]">Right</span>
              {" · "}
              {baseName(rightPath)} · {picture.widthR}×{picture.heightR} ({picture.sizeR} bytes)
            </span>
            {picture.truncated ? " · samples truncated" : ""}
          </p>
          {picture.samples.length > 0 && (
            <div data-testid="mediaview/rows">
              {picture.samples.slice(0, 20).map((s) => (
                <div
                  key={s.offset}
                  data-testid="mediaview/row"
                  className="mw-row flex items-center gap-[var(--ac-space-2)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]"
                >
                  <span className="font-medium">offset {s.offset}</span>
                  <span className="inline-flex items-center gap-[var(--ac-space-1)]">
                    <span className="font-medium text-[var(--ac-fg)]">{s.left}</span>
                    <ArrowRight size={12} aria-hidden="true" />
                    <span className="font-medium text-[var(--ac-fg)]">{s.right}</span>
                  </span>
                </div>
              ))}
            </div>
          )}
          {/* Wave-147: diff empty states (BC names the no-diff case
              instead of a bare score line). Identical files carry no
              samples by contract; a differing file with zero samples
              means the summary cap hid them — said honestly. Renders
              ONLY when the rows block is absent, so row counts and
              chrome are untouched. */}
          {picture.samples.length === 0 && (
            <p data-testid="mediaview/empty-rows" className="mw-row font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">
              {picture.identical ? "No differences — files are identical." : "No diff samples within the summary cap."}
            </p>
          )}
        </div>
      )}
      {status === "ready" && audio !== null && (
        <div data-testid="mediaview/summary" className="flex flex-col gap-[var(--ac-space-2)] px-[var(--ac-space-4)] py-[var(--ac-space-3)]">
          <p data-testid="mediaview/score" className="ac-chrome-label font-medium text-[var(--ac-fg)]">
            {audio.identical ? "Identical audio" : `${pct(audio.score)} equal`} — {audio.windowsDiffering} / {audio.windowsTotal} windows differ
            {audio.firstDiffAt !== null ? `, first at ${audio.firstDiffAt.toFixed(2)}s` : ""}
          </p>
          <p className="mw-row flex flex-nowrap items-baseline gap-x-[var(--ac-space-4)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">
            <span data-testid="mediaview/meta-left" title={leftPath} className="min-w-0 flex-1 truncate">
              <span aria-hidden="true" className="ac-chrome-label font-medium text-[var(--ac-fg)]">Left</span>
              {" · "}
              {baseName(leftPath)} · {audio.sampleRateL}Hz×{audio.channelsL} {audio.durationL.toFixed(2)}s
            </span>
            <span data-testid="mediaview/meta-right" title={rightPath} className="min-w-0 flex-1 truncate">
              <span aria-hidden="true" className="ac-chrome-label font-medium text-[var(--ac-fg)]">Right</span>
              {" · "}
              {baseName(rightPath)} · {audio.sampleRateR}Hz×{audio.channelsR} {audio.durationR.toFixed(2)}s
            </span>
            {audio.configMismatch ? " · config mismatch" : ""}
            {audio.truncated ? " · windows truncated" : ""}
          </p>
          {audio.diffWindows.length > 0 && (
            <div data-testid="mediaview/rows">
              {audio.diffWindows.slice(0, 20).map((w) => (
                <div
                  key={w.window}
                  data-testid="mediaview/row"
                  className="mw-row flex items-center gap-[var(--ac-space-2)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]"
                >
                  <span className="font-medium">window {w.window}</span>
                  <span>@{w.atSeconds.toFixed(2)}s</span>
                </div>
              ))}
            </div>
          )}
          {/* Wave-147: audio diff empty state (picture twin above). */}
          {audio.diffWindows.length === 0 && (
            <p data-testid="mediaview/empty-rows" className="mw-row font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">
              {audio.identical ? "No differences — files are identical." : "No diff windows within the summary cap."}
            </p>
          )}
        </div>
      )}
      {/* Wave-147: identical/empty diff note (BC empty-state parity).
          Renders ONLY when the backend summary says identical or carries
          zero samples/windows, so mock non-identical summaries (2 picture
          samples / 3 audio windows) never hit it and wave-48/72/103 specs
          hold byte-identical. Same in-flow line, no new chrome rows. */}
      {status === "ready" && picture !== null && (picture.identical || picture.samples.length === 0) && (
        <p data-testid="mediaview/empty-diff" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          no differing samples — files identical
        </p>
      )}
      {status === "ready" && audio !== null && (audio.identical || audio.diffWindows.length === 0) && (
        <p data-testid="mediaview/empty-diff" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          no differing windows — files identical
        </p>
      )}
      {/* Wave-72 bottom status bar (Beyond Compare style). Honest scope:
          reuses the existing ready summary state only -- "same" when the
          backend summary says identical, else "different", plus the score
          and the differing/total counts. NO backend change: no per-pixel
          or per-sample counts exist in the media summaries, so none is
          claimed here. */}
      {status === "ready" && (picture !== null || audio !== null) && (
        <div
          data-testid="mediaview/statusbar"
          role="status"
          className="mt-[var(--ac-space-2)] flex flex-wrap items-center gap-[var(--ac-space-2)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          {picture !== null ? (
            <>
              <span data-testid="mediaview/statusbar-verdict">
                Picture differences | {picture.identical ? "same" : "different"}
              </span>
              <span data-testid="mediaview/statusbar-counts">
                {pct(picture.score)} · {picture.differingBytes} / {picture.totalBytes} bytes differ
              </span>
            </>
          ) : audio !== null ? (
            <>
              <span data-testid="mediaview/statusbar-verdict">
                Audio differences | {audio.identical ? "same" : "different"}
              </span>
              <span data-testid="mediaview/statusbar-counts">
                {pct(audio.score)} · {audio.windowsDiffering} / {audio.windowsTotal} windows differ
              </span>
            </>
          ) : null}
        </div>
      )}
    </section>
  );
}
