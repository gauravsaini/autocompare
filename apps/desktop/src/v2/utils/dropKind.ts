import type { SessionKind } from "../types";

export function inferDropKind(paths: readonly string[]): SessionKind {
  const first = (paths[0] ?? "").toLowerCase();
  if (first.endsWith(".csv") || first.endsWith(".tsv")) return "table";
  if (
    first.endsWith(".png") ||
    first.endsWith(".jpg") ||
    first.endsWith(".jpeg") ||
    first.endsWith(".wav") ||
    first.endsWith(".mp3")
  ) {
    return "media";
  }
  if (
    first.endsWith(".bin") ||
    first.endsWith(".wasm") ||
    first.endsWith(".exe") ||
    first.endsWith(".dylib")
  ) {
    return "hex";
  }
  const base = first.split("/").pop() ?? "";
  if (first.endsWith("/") || (base !== "" && !base.includes("."))) {
    return "folder";
  }
  return "text";
}
