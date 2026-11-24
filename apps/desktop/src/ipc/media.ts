/**
 * Media-compare IPC surface (Wave 1d).
 *
 * Mirrors the `serde` DTOs in
 * `src-tauri/src/commands/media.rs` (`rename_all = "camelCase"` on the
 * Rust side, so the JSON field names below match exactly). All calls go
 * through the typed wrapper in `./client`, which serves fixtures under
 * `VITE_MOCK_IPC=1`.
 *
 * Bounded-payload rule (locked): picture/audio comparisons return scalar
 * metrics plus downsampled diff stats only -- never full bitmaps or
 * sample buffers. Sample arrays are capped at 256 entries server-side;
 * longer runs set `truncated`.
 *
 * Wave 1R handoff: the command names below (`picture_compare`,
 * `audio_compare`) must be registered in the `lib.rs`
 * `generate_handler![...]` table for production; under
 * `VITE_MOCK_IPC=1` the client fixture below answers instead.
 */

import { invoke } from "./client";

/** Rust mirror: `commands::media::ByteDiffSample`. */
export interface ByteDiffSample {
  readonly offset: number;
  readonly left: number;
  readonly right: number;
}

/** Rust mirror: `commands::media::PictureSummary` (bounded, no bitmaps). */
export interface PictureSummary {
  readonly receipt: string;
  readonly identical: boolean;
  /** Sniffed container: `"png"` or `"unknown"`. */
  readonly format: string;
  readonly widthL: number;
  readonly heightL: number;
  readonly widthR: number;
  readonly heightR: number;
  readonly sizeL: number;
  readonly sizeR: number;
  /** Fraction of equal bytes over the longer file, in `[0.0, 1.0]`. */
  readonly score: number;
  readonly differingBytes: number;
  readonly totalBytes: number;
  /** Downsampled differing offsets (max 256). */
  readonly samples: readonly ByteDiffSample[];
  /** True when differing positions exceeded the sample cap. */
  readonly truncated: boolean;
}

/** Rust mirror: `commands::media::AudioDiffWindow`. */
export interface AudioDiffWindow {
  /** 10 ms window index into the common duration. */
  readonly window: number;
  /** Window start in seconds. */
  readonly atSeconds: number;
}

/** Rust mirror: `commands::media::AudioSummary` (bounded, no samples). */
export interface AudioSummary {
  readonly receipt: string;
  readonly identical: boolean;
  /** True when rates/channels/depth differ; windows are then empty. */
  readonly configMismatch: boolean;
  readonly sampleRateL: number;
  readonly sampleRateR: number;
  readonly channelsL: number;
  readonly channelsR: number;
  readonly durationL: number;
  readonly durationR: number;
  /** Applied amplitude tolerance, echoed back. */
  readonly tolerance: number;
  readonly windowsTotal: number;
  readonly windowsDiffering: number;
  readonly firstDiffAt: number | null;
  readonly score: number;
  /** Downsampled differing windows (max 256). */
  readonly diffWindows: readonly AudioDiffWindow[];
  /** True when differing windows exceeded the sample cap. */
  readonly truncated: boolean;
}

/**
 * Compare two picture files; resolves a bounded summary (scalar metrics
 * plus downsampled diff stats, never full bitmaps).
 */
export function pictureCompare(left: string, right: string): Promise<PictureSummary> {
  return invoke<PictureSummary>("picture_compare", { left, right });
}

/**
 * Compare two audio files with an amplitude tolerance (`tolerance`
 * omitted means exact comparison); resolves a bounded summary.
 */
export function audioCompare(
  left: string,
  right: string,
  tolerance?: number,
): Promise<AudioSummary> {
  return invoke<AudioSummary>("audio_compare", {
    left,
    right,
    tolerance: tolerance ?? null,
  });
}
