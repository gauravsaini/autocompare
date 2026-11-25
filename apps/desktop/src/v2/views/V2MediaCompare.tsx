import React, { useCallback, useEffect, useState } from "react";
import { AudioWaveform, Image as ImageIcon } from "lucide-react";
import { audioCompare, pictureCompare } from "../../ipc/media";
import type { AudioSummary, PictureSummary } from "../../ipc/media";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import { isMissingFixtureError } from "../utils/demoGuard";
import type { V2StatusInfo, V2ViewCommands } from "../types";

export interface V2MediaCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

type MediaKind = "picture" | "audio";

function inferMediaKind(path: string): MediaKind {
  const lower = path.toLowerCase();
  if (lower.endsWith(".wav") || lower.endsWith(".mp3")) return "audio";
  return "picture";
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
  const [pic, setPic] = useState<PictureSummary | null>(null);
  const [aud, setAud] = useState<AudioSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runCompare = useCallback(
    async (mKind: MediaKind, l: string, r: string, tol: number): Promise<void> => {
      setLoading(true);
      setError(null);
      try {
        if (mKind === "picture") {
          const lEff = l || "tools/parity-harness/fixtures/pic_left.png";
          const rEff = r || "tools/parity-harness/fixtures/pic_right.png";
          const res = await pictureCompare(lEff, rEff, tol > 0 ? tol : undefined);
          setPic(res);
          setAud(null);
        } else {
          const lEff = l || "tools/parity-harness/fixtures/aud_left.wav";
          const rEff = r || "tools/parity-harness/fixtures/aud_right.wav";
          const res = await audioCompare(lEff, rEff, tol > 0 ? tol : undefined);
          setAud(res);
          setPic(null);
        }
      } catch (e) {
        if (!l && !r && isMissingFixtureError(e)) {
          setPic(null);
          setAud(null);
        } else {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
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
        center: `Similarity ${(pic.score * 100).toFixed(2)}% · ${pic.format.toUpperCase()}`,
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
  }, [onStatusChange, kind, pic, aud]);

  const scorePct = kind === "picture" && pic ? pic.score * 100 : kind === "audio" && aud ? aud.score * 100 : 0;

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

        <label className="flex items-center gap-2 text-[12px] text-[var(--v2-fg-secondary)]">
          <span>Tolerance</span>
          <input
            type="range"
            min={0}
            max={kind === "picture" ? 32 : 0.1}
            step={kind === "picture" ? 1 : 0.005}
            value={tolerance}
            onChange={(e) => setTolerance(Number(e.target.value))}
            className="w-28 accent-[var(--v2-accent)]"
          />
          <span className="v2-mono w-12 text-right text-[11px] text-[var(--v2-fg)]">
            {kind === "picture" ? tolerance : tolerance.toFixed(3)}
          </span>
        </label>
      </div>

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Main Media Analysis Surface */}
      <div className="min-h-0 flex-1 overflow-y-auto bg-[var(--v2-bg)] p-5">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
            Analyzing {kind} streams…
          </div>
        ) : kind === "picture" && pic ? (
          <div className="mx-auto max-w-4xl space-y-4">
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
