import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AudioWaveform, Image as ImageIcon, ZoomIn, ZoomOut } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { audioCompare, pictureCompare } from "../../ipc/media";
import type { AudioSummary, PictureSummary } from "../../ipc/media";
import { clearActiveTrace, setActiveTrace } from "../../ipc/client";
import { endSpan, isTelemetryEnabled, startRootSpan, type Span } from "../../observability/tracer";
import { emitFileWatchTransition } from "../utils/fileWatch";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2ReloadPromptBanner from "../components/V2ReloadPromptBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import { isMissingFixtureError } from "../utils/demoGuard";
import { useExternalChangeWatch } from "../hooks/useExternalChangeWatch";
import type { V2StatusInfo, V2ViewCommands } from "../types";

export interface V2MediaCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

type MediaKind = "picture" | "audio";
type PictureViewMode = "sideBySide" | "wipe" | "onion" | "diff";

function inferMediaKind(path: string): MediaKind {
  const lower = path.toLowerCase();
  if (lower.endsWith(".wav") || lower.endsWith(".mp3")) return "audio";
  return "picture";
}

function makePreviewSvg(label: string, accent: string, altAccent: string, dims: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180" viewBox="0 0 320 180">
    <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="${accent}"/><stop offset="100%" stop-color="${altAccent}"/></linearGradient></defs>
    <rect width="320" height="180" fill="url(#g)" rx="8"/>
    <circle cx="96" cy="86" r="34" fill="rgba(255,255,255,0.22)"/>
    <rect x="148" y="54" width="124" height="64" rx="8" fill="rgba(255,255,255,0.18)"/>
    <text x="16" y="30" fill="#ffffff" font-family="system-ui,sans-serif" font-size="13" font-weight="600">${label}</text>
    <text x="16" y="162" fill="rgba(255,255,255,0.85)" font-family="monospace" font-size="11">${dims}</text>
  </svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function resolveImageUrl(path: string, fallbackSvg: string): string {
  const trimmed = path.trim();
  if (!trimmed) return fallbackSvg;
  if (/^(https?:|data:|blob:)/i.test(trimmed)) return trimmed;
  const hasTauri =
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
  if (hasTauri) {
    try {
      return convertFileSrc(trimmed);
    } catch {
      return fallbackSvg;
    }
  }
  return fallbackSvg;
}

export default function V2MediaCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2MediaCompareProps): React.JSX.Element {
  const [kind, setKind] = useState<MediaKind>(() => inferMediaKind(leftPath || rightPath));
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [tolerance, setTolerance] = useState(0);
  const [viewMode, setViewMode] = useState<PictureViewMode>("wipe");
  const [wipePct, setWipePct] = useState(50);
  const [onionOpacity, setOnionOpacity] = useState(50);
  const [zoomPct, setZoomPct] = useState(100);
  const [leftImgErr, setLeftImgErr] = useState(false);
  const [rightImgErr, setRightImgErr] = useState(false);
  const [pic, setPic] = useState<PictureSummary | null>(null);
  const [aud, setAud] = useState<AudioSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // ODD Phase 4 Leaf A: ambient trace of the last compare, so reload prompts
  // link to the same trace after the root span ends.
  const lastTraceRef = useRef<{ trace_id: string; span_id: string; sampled: boolean } | null>(null);
  const watchedPaths = useMemo(
    () => [leftDraft.trim(), rightDraft.trim()].filter((p) => p.length > 0),
    [leftDraft, rightDraft],
  );
  const { changedPaths, dismiss: dismissReload, resetBaseline } = useExternalChangeWatch({
    paths: watchedPaths,
    enabled: !loading && watchedPaths.length > 0,
    onAutoReload: () => void runCompare(kind, leftDraft, rightDraft, tolerance),
  });

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
    setLeftImgErr(false);
    setRightImgErr(false);
  }, [leftPath, rightPath]);

  // ODD Phase 4 Leaf A: file-watch reload prompts emit explicit
  // state-transition events linked to the ambient (last compare) trace.
  const promptEmittedRef = useRef<string>("");
  useEffect(() => {
    if (changedPaths.length === 0) {
      promptEmittedRef.current = "";
      return;
    }
    const key = changedPaths.join("\0");
    if (promptEmittedRef.current === key) return;
    promptEmittedRef.current = key;
    emitFileWatchTransition("media", "prompt_shown", { paths: changedPaths.length }, lastTraceRef.current);
  }, [changedPaths]);

  const runCompare = useCallback(
    async (mKind: MediaKind, l: string, r: string, tol: number): Promise<void> => {
      // ODD Phase 4 Leaf A: one root span per compare trigger
      // (op media.picture_compare / media.audio_compare), following the exact
      // V2TextCompare pattern. Telemetry-off runs identically.
      const op = mKind === "picture" ? "media.picture_compare" : "media.audio_compare";
      const obs: Span | null = isTelemetryEnabled()
        ? startRootSpan({ op, component: "ts.ui/MediaView", view_id: "media" })
        : null;
      if (obs !== null) {
        setActiveTrace(obs.context());
        lastTraceRef.current = obs.context();
      }
      setLoading(true);
      setError(null);
      try {
        if (mKind === "picture") {
          const lEff = l || "tools/parity-harness/fixtures/pic_left.png";
          const rEff = r || "tools/parity-harness/fixtures/pic_right.png";
          const res = await pictureCompare(lEff, rEff, tol > 0 ? tol : undefined);
          setPic(res);
          setAud(null);
          if (obs !== null) {
            obs.set({
              rows: res.samples.length,
              hunks: res.differingBytes,
              equal: res.identical,
              score: res.score,
              session: res.receipt,
            });
          }
        } else {
          const lEff = l || "tools/parity-harness/fixtures/aud_left.wav";
          const rEff = r || "tools/parity-harness/fixtures/aud_right.wav";
          const res = await audioCompare(lEff, rEff, tol > 0 ? tol : undefined);
          setAud(res);
          setPic(null);
          if (obs !== null) {
            obs.set({
              rows: res.diffWindows.length,
              hunks: res.windowsDiffering,
              equal: res.identical,
              score: res.score,
              session: res.receipt,
            });
          }
        }
      } catch (e) {
        if (obs !== null) {
          obs.error({
            kind: "media_compare_failed",
            component: "ts.ui/MediaView",
            message: e instanceof Error ? e.message : String(e),
          });
        }
        if (!l && !r && isMissingFixtureError(e)) {
          setPic(null);
          setAud(null);
        } else {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
        if (obs !== null) {
          clearActiveTrace();
          endSpan(obs);
        }
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    void runCompare(kind, leftPath, rightPath, tolerance);
  }, [kind, leftPath, rightPath, tolerance, runCompare]);

  useEffect(() => {
    onRegisterCommands({
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runCompare(kind, leftDraft, rightDraft, tolerance),
    });
  }, [onRegisterCommands, onPathsChange, leftDraft, rightDraft, runCompare, kind, tolerance]);

  useEffect(() => {
    if (kind === "picture" && pic) {
      onStatusChange({
        left: pic.identical ? "Pictures are identical" : `${pic.differingBytes} differing bytes`,
        center: `Similarity ${(pic.score * 100).toFixed(2)}% · ${pic.format.toUpperCase()} · Zoom ${zoomPct}%`,
        right: `${pic.widthL}×${pic.heightL} vs ${pic.widthR}×${pic.heightR}`,
      });
    } else if (kind === "audio" && aud) {
      onStatusChange({
        left: aud.identical
          ? "Audio streams are identical"
          : aud.configMismatch
            ? "Audio format mismatch"
            : `${aud.windowsDiffering} differing windows`,
        center: `Similarity ${(aud.score * 100).toFixed(2)}% · tol ${aud.tolerance}`,
        right: `${aud.durationL.toFixed(2)}s / ${aud.durationR.toFixed(2)}s`,
      });
    } else {
      onStatusChange({ left: "Ready", center: kind === "picture" ? "Picture Compare" : "Audio Compare", right: "" });
    }
  }, [onStatusChange, kind, pic, aud, zoomPct]);

  const scorePct = kind === "picture" && pic ? pic.score * 100 : kind === "audio" && aud ? aud.score * 100 : 0;

  const leftFallback = useMemo(
    () => makePreviewSvg("Left Picture", "#2563eb", "#1d4ed8", pic ? `${pic.widthL}×${pic.heightL} px` : "Preview"),
    [pic],
  );
  const rightFallback = useMemo(
    () => makePreviewSvg("Right Picture", "#059669", "#047857", pic ? `${pic.widthR}×${pic.heightR} px` : "Preview"),
    [pic],
  );
  const leftSrc = leftImgErr ? leftFallback : resolveImageUrl(leftDraft, leftFallback);
  const rightSrc = rightImgErr ? rightFallback : resolveImageUrl(rightDraft, rightFallback);

  return (
    <div data-testid="v2/media-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder={`Left ${kind} path…`}
        rightPlaceholder={`Right ${kind} path…`}
      />

      {/* Media Mode + Tolerance Bar */}
      <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
        <div className="flex flex-wrap items-center gap-2">
          <div className="inline-flex rounded-[6px] bg-[var(--v2-seg-track)] p-0.5">
            {(
              [
                ["picture", "Picture", ImageIcon],
                ["audio", "Audio", AudioWaveform],
              ] as const
            ).map(([m, label, Icon]) => {
              const active = kind === m;
              return (
                <button
                  key={m}
                  type="button"
                  onClick={() => setKind(m)}
                  className={`inline-flex h-[22px] items-center gap-1.5 rounded-[5px] px-2.5 text-[11.5px] font-medium transition ${
                    active
                      ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                      : "text-[var(--v2-fg-secondary)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  <Icon size={12} />
                  {label}
                </button>
              );
            })}
          </div>

          {kind === "picture" ? (
            <div className="inline-flex rounded-[6px] bg-[var(--v2-seg-track)] p-0.5">
              {(
                [
                  ["sideBySide", "Side-by-Side"],
                  ["wipe", "Slider Wipe"],
                  ["onion", "Onion Skin"],
                  ["diff", "Pixel Diff"],
                ] as const
              ).map(([mode, label]) => (
                <button
                  key={mode}
                  type="button"
                  data-testid={`v2/media-mode-${mode}`}
                  onClick={() => setViewMode(mode)}
                  className={`h-[22px] rounded-[5px] px-2 text-[11px] font-medium transition ${
                    viewMode === mode
                      ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                      : "text-[var(--v2-fg-secondary)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          ) : null}
        </div>

        <div className="flex flex-wrap items-center gap-3">
          {kind === "picture" ? (
            <div className="flex items-center gap-1">
              <button
                type="button"
                title="Zoom Out"
                onClick={() => setZoomPct((z) => Math.max(50, z - 25))}
                className="v2-btn h-[22px] px-1.5"
              >
                <ZoomOut size={12} />
              </button>
              <button
                type="button"
                title="Reset Zoom"
                onClick={() => setZoomPct(100)}
                className="v2-btn v2-mono h-[22px] px-2 text-[11px]"
              >
                {zoomPct}%
              </button>
              <button
                type="button"
                title="Zoom In"
                onClick={() => setZoomPct((z) => Math.min(400, z + 25))}
                className="v2-btn h-[22px] px-1.5"
              >
                <ZoomIn size={12} />
              </button>
            </div>
          ) : null}

          <label className="flex items-center gap-2 text-[12px] text-[var(--v2-fg-secondary)]">
            <span>Tolerance</span>
            <input
              type="range"
              min={0}
              max={kind === "picture" ? 32 : 0.1}
              step={kind === "picture" ? 1 : 0.005}
              value={tolerance}
              onChange={(e) => setTolerance(Number(e.target.value))}
              className="w-24 accent-[var(--v2-accent)]"
            />
            <span className="v2-mono w-10 text-right text-[11px] text-[var(--v2-fg)]">
              {kind === "picture" ? tolerance : tolerance.toFixed(3)}
            </span>
          </label>
        </div>
      </div>

      <V2ReloadPromptBanner
        changedPaths={changedPaths}
        onReload={() => {
          emitFileWatchTransition("media", "reload_accepted", { paths: changedPaths.length }, lastTraceRef.current);
          void resetBaseline();
          void runCompare(kind, leftDraft, rightDraft, tolerance);
        }}
        onDismiss={() => {
          emitFileWatchTransition("media", "reload_dismissed", { paths: changedPaths.length }, lastTraceRef.current);
          dismissReload();
        }}
      />
      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Main Media Analysis Surface */}
      <div className="min-h-0 flex-1 overflow-y-auto bg-[var(--v2-bg)] p-5">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
            Analyzing {kind} streams…
          </div>
        ) : kind === "picture" && pic ? (
          <div className="mx-auto max-w-4xl space-y-4">
            {/* Interactive Visual Comparison Stage */}
            <div
              data-testid="v2/media-stage"
              className="overflow-hidden rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)]"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-3.5 py-2 text-[11.5px]">
                <span className="font-semibold text-[var(--v2-fg-secondary)]">
                  Visual Compare — {viewMode === "sideBySide" ? "Side-by-Side" : viewMode === "wipe" ? "Slider Wipe" : viewMode === "onion" ? "Onion Skin" : "Pixel Diff"}
                </span>
                {viewMode === "wipe" ? (
                  <label className="flex items-center gap-2 text-[11px] text-[var(--v2-fg-muted)]">
                    <span>Wipe {wipePct}%</span>
                    <input
                      type="range"
                      min={0}
                      max={100}
                      value={wipePct}
                      onChange={(e) => setWipePct(Number(e.target.value))}
                      className="w-32 accent-[var(--v2-accent)]"
                    />
                  </label>
                ) : viewMode === "onion" ? (
                  <label className="flex items-center gap-2 text-[11px] text-[var(--v2-fg-muted)]">
                    <span>Right Opacity {onionOpacity}%</span>
                    <input
                      type="range"
                      min={0}
                      max={100}
                      value={onionOpacity}
                      onChange={(e) => setOnionOpacity(Number(e.target.value))}
                      className="w-32 accent-[var(--v2-accent)]"
                    />
                  </label>
                ) : null}
              </div>

              <div className="flex max-h-[340px] items-center justify-center overflow-auto bg-[var(--v2-bg)] p-4">
                {viewMode === "sideBySide" ? (
                  <div className="grid w-full grid-cols-2 gap-3">
                    <div className="flex flex-col items-center gap-1 overflow-hidden">
                      <img
                        src={leftSrc}
                        alt="Left comparison"
                        onError={() => setLeftImgErr(true)}
                        style={{ transform: `scale(${zoomPct / 100})`, transformOrigin: "top center" }}
                        className="max-h-[240px] rounded border border-[var(--v2-border)] object-contain"
                      />
                      <span className="text-[10.5px] text-[var(--v2-fg-muted)]">Left</span>
                    </div>
                    <div className="flex flex-col items-center gap-1 overflow-hidden">
                      <img
                        src={rightSrc}
                        alt="Right comparison"
                        onError={() => setRightImgErr(true)}
                        style={{ transform: `scale(${zoomPct / 100})`, transformOrigin: "top center" }}
                        className="max-h-[240px] rounded border border-[var(--v2-border)] object-contain"
                      />
                      <span className="text-[10.5px] text-[var(--v2-fg-muted)]">Right</span>
                    </div>
                  </div>
                ) : (
                  <div
                    className="relative inline-block overflow-hidden rounded border border-[var(--v2-border)]"
                    style={{ transform: `scale(${zoomPct / 100})`, transformOrigin: "center" }}
                  >
                    <img
                      src={leftSrc}
                      alt="Left base"
                      onError={() => setLeftImgErr(true)}
                      className="block max-h-[260px] max-w-full object-contain"
                    />
                    <img
                      src={rightSrc}
                      alt="Right overlay"
                      onError={() => setRightImgErr(true)}
                      style={
                        viewMode === "wipe"
                          ? { clipPath: `inset(0 0 0 ${wipePct}%)` }
                          : viewMode === "onion"
                            ? { opacity: onionOpacity / 100 }
                            : { mixBlendMode: "difference" }
                      }
                      className="pointer-events-none absolute inset-0 h-full w-full object-contain"
                    />
                    {viewMode === "wipe" ? (
                      <div
                        style={{ left: `${wipePct}%` }}
                        className="pointer-events-none absolute inset-y-0 w-0.5 -translate-x-1/2 bg-[var(--v2-accent)] shadow"
                      />
                    ) : null}
                  </div>
                )}
              </div>
            </div>

            {/* Similarity Header Card */}
            <div className="rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-4">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[11px] font-semibold uppercase tracking-wider text-[var(--v2-fg-muted)]">
                    Picture Similarity ({pic.format.toUpperCase()})
                  </div>
                  <div className="mt-1 text-[24px] font-semibold tracking-tight text-[var(--v2-fg)]">
                    {scorePct.toFixed(2)}%
                    <span className="ml-2 text-[12px] font-normal text-[var(--v2-fg-muted)]">
                      {pic.identical ? "Exact match" : `${pic.differingBytes.toLocaleString()} / ${pic.totalBytes.toLocaleString()} bytes differ`}
                    </span>
                  </div>
                </div>
                <span
                  className="rounded-full px-2.5 py-1 text-[11px] font-semibold"
                  style={{
                    backgroundColor: pic.identical ? "var(--v2-ins-bg)" : "var(--v2-chg-bg)",
                    color: pic.identical ? "var(--v2-ins-fg)" : "var(--v2-chg-fg)",
                  }}
                >
                  {pic.identical ? "Identical" : "Differences Found"}
                </span>
              </div>
              <div className="mt-3 h-2 w-full overflow-hidden rounded-full bg-[var(--v2-seg-track)]">
                <div
                  className="h-full rounded-full transition-all"
                  style={{
                    width: `${Math.max(0, Math.min(100, scorePct))}%`,
                    backgroundColor: pic.identical ? "var(--v2-ruler-ins)" : "var(--v2-accent)",
                  }}
                />
              </div>
            </div>

            {/* Dual Side Metadata */}
            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-3.5">
                <div className="text-[11px] font-semibold uppercase text-[var(--v2-fg-muted)]">Left Picture</div>
                <div className="v2-mono mt-2 grid grid-cols-2 gap-y-1 text-[12px]">
                  <span className="text-[var(--v2-fg-muted)]">Dimensions</span>
                  <span className="text-right">{pic.widthL} × {pic.heightL} px</span>
                  <span className="text-[var(--v2-fg-muted)]">File Size</span>
                  <span className="text-right">{pic.sizeL.toLocaleString()} B</span>
                </div>
              </div>
              <div className="rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-3.5">
                <div className="text-[11px] font-semibold uppercase text-[var(--v2-fg-muted)]">Right Picture</div>
                <div className="v2-mono mt-2 grid grid-cols-2 gap-y-1 text-[12px]">
                  <span className="text-[var(--v2-fg-muted)]">Dimensions</span>
                  <span className="text-right">{pic.widthR} × {pic.heightR} px</span>
                  <span className="text-[var(--v2-fg-muted)]">File Size</span>
                  <span className="text-right">{pic.sizeR.toLocaleString()} B</span>
                </div>
              </div>
            </div>

            {/* Downsampled Diff Offsets */}
            {pic.samples.length > 0 ? (
              <div className="overflow-hidden rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)]">
                <div className="border-b border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-3.5 py-2 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
                  Differing Byte Samples ({pic.samples.length}{pic.truncated ? "+" : ""})
                </div>
                <div className="v2-mono max-h-[220px] overflow-y-auto text-[11.5px]">
                  {pic.samples.slice(0, 64).map((s) => (
                    <div
                      key={s.offset}
                      className="grid grid-cols-3 border-b border-[var(--v2-border-subtle)]/50 px-3.5 py-1"
                    >
                      <span className="text-[var(--v2-fg-muted)]">0x{s.offset.toString(16).toUpperCase().padStart(8, "0")}</span>
                      <span style={{ color: "var(--v2-del-fg)" }}>Left: 0x{s.left.toString(16).toUpperCase().padStart(2, "0")}</span>
                      <span style={{ color: "var(--v2-ins-fg)" }}>Right: 0x{s.right.toString(16).toUpperCase().padStart(2, "0")}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}
          </div>
        ) : kind === "audio" && aud ? (
          <div className="mx-auto max-w-4xl space-y-4">
            <div className="rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-4">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[11px] font-semibold uppercase tracking-wider text-[var(--v2-fg-muted)]">
                    Audio Stream Similarity
                  </div>
                  <div className="mt-1 text-[24px] font-semibold tracking-tight text-[var(--v2-fg)]">
                    {scorePct.toFixed(2)}%
                    <span className="ml-2 text-[12px] font-normal text-[var(--v2-fg-muted)]">
                      {aud.windowsDiffering} / {aud.windowsTotal} windows differ
                    </span>
                  </div>
                </div>
                <span
                  className="rounded-full px-2.5 py-1 text-[11px] font-semibold"
                  style={{
                    backgroundColor: aud.identical ? "var(--v2-ins-bg)" : "var(--v2-chg-bg)",
                    color: aud.identical ? "var(--v2-ins-fg)" : "var(--v2-chg-fg)",
                  }}
                >
                  {aud.identical ? "Identical" : aud.configMismatch ? "Format Mismatch" : "Differences Found"}
                </span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-3.5">
                <div className="text-[11px] font-semibold uppercase text-[var(--v2-fg-muted)]">Left Audio</div>
                <div className="v2-mono mt-2 grid grid-cols-2 gap-y-1 text-[12px]">
                  <span className="text-[var(--v2-fg-muted)]">Sample Rate</span>
                  <span className="text-right">{aud.sampleRateL.toLocaleString()} Hz</span>
                  <span className="text-[var(--v2-fg-muted)]">Channels</span>
                  <span className="text-right">{aud.channelsL}</span>
                  <span className="text-[var(--v2-fg-muted)]">Duration</span>
                  <span className="text-right">{aud.durationL.toFixed(2)} s</span>
                </div>
              </div>
              <div className="rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-3.5">
                <div className="text-[11px] font-semibold uppercase text-[var(--v2-fg-muted)]">Right Audio</div>
                <div className="v2-mono mt-2 grid grid-cols-2 gap-y-1 text-[12px]">
                  <span className="text-[var(--v2-fg-muted)]">Sample Rate</span>
                  <span className="text-right">{aud.sampleRateR.toLocaleString()} Hz</span>
                  <span className="text-[var(--v2-fg-muted)]">Channels</span>
                  <span className="text-right">{aud.channelsR}</span>
                  <span className="text-[var(--v2-fg-muted)]">Duration</span>
                  <span className="text-right">{aud.durationR.toFixed(2)} s</span>
                </div>
              </div>
            </div>

            {aud.diffWindows.length > 0 ? (
              <div className="overflow-hidden rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)]">
                <div className="border-b border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-3.5 py-2 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
                  Differing 10ms Windows ({aud.diffWindows.length}{aud.truncated ? "+" : ""})
                </div>
                <div className="flex flex-wrap gap-1.5 p-3">
                  {aud.diffWindows.slice(0, 60).map((w) => (
                    <span
                      key={w.window}
                      className="v2-mono rounded bg-[var(--v2-chg-bg)] px-1.5 py-0.5 text-[11px] text-[var(--v2-chg-fg)]"
                    >
                      {w.atSeconds.toFixed(2)}s
                    </span>
                  ))}
                </div>
              </div>
            ) : null}
          </div>
        ) : (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
            Select two {kind} files to compare.
          </div>
        )}
      </div>
    </div>
  );
}
