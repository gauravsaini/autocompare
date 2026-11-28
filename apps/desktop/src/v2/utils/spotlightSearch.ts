import type { SessionKind } from "../types";

export interface ParsedSpotlightPaths {
  readonly left: string;
  readonly right: string;
  readonly inferredKind: SessionKind;
}

const PATH_SEPARATOR_TOKENS = new Set(["vs", "vs.", "<->", "->", "<=>", "|", ","]);

/**
 * Splits a Spotlight query into tokens while preserving single- or double-quoted paths.
 */
export function tokenizeSpotlightInput(raw: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let quote: '"' | "'" | null = null;

  for (let i = 0; i < raw.length; i += 1) {
    const ch = raw[i];
    if (quote) {
      if (ch === quote) {
        quote = null;
      } else {
        current += ch;
      }
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (/\s/.test(ch)) {
      if (current.length > 0) {
        tokens.push(current);
        current = "";
      }
      continue;
    }
    current += ch;
  }
  if (current.length > 0) tokens.push(current);
  return tokens;
}

/**
 * Returns true if a token resembles a filesystem path or filename with extension.
 */
export function looksLikePathToken(token: string): boolean {
  const t = token.trim();
  if (!t) return false;
  if (
    t.startsWith("/") ||
    t.startsWith("~/") ||
    t.startsWith("./") ||
    t.startsWith("../") ||
    /^[a-zA-Z]:[\\/]/.test(t)
  ) {
    return true;
  }
  if (t.includes("/") || t.includes("\\")) {
    return !t.startsWith("http://") && !t.startsWith("https://");
  }
  // Filename with a realistic extension (e.g. config.json, app.tsx, data.csv)
  return /^[^.\s][^/\s]*\.[a-zA-Z0-9]{1,8}$/.test(t);
}

function defaultInferKind(paths: readonly string[]): SessionKind {
  const first = (paths[0] ?? "").toLowerCase();
  if (!first) return "text";
  if (first.endsWith(".json") || first.endsWith(".yaml") || first.endsWith(".yml")) {
    return "structured";
  }
  if (first.endsWith(".toml") || first.endsWith(".xml")) {
    return "tomlXml";
  }
  if (first.endsWith(".csv") || first.endsWith(".tsv")) {
    return "table";
  }
  if (first.endsWith(".bin") || first.endsWith(".dat") || first.endsWith(".exe") || first.endsWith(".wasm")) {
    return "hex";
  }
  if (
    first.endsWith(".png") ||
    first.endsWith(".jpg") ||
    first.endsWith(".jpeg") ||
    first.endsWith(".webp") ||
    first.endsWith(".gif") ||
    first.endsWith(".mp3") ||
    first.endsWith(".wav")
  ) {
    return "media";
  }
  const base = first.split(/[\\/]/).pop() ?? "";
  if (first.endsWith("/") || first.endsWith("\\") || (base !== "" && !base.includes("."))) {
    return "folder";
  }
  return "text";
}

/**
 * Parses pasted left/right paths from the Home Spotlight search input.
 * Supports:
 *  - `/path/a.txt /path/b.txt`
 *  - `/path/a.txt vs /path/b.txt` or `/path/a.txt <-> /path/b.txt`
 *  - `"/path/with spaces/a.json" "/path/with spaces/b.json"`
 *  - single path `/path/a.csv` (opens comparator with left pre-filled)
 */
export function parseSpotlightPaths(
  raw: string,
  inferKind: (paths: readonly string[]) => SessionKind = defaultInferKind,
): ParsedSpotlightPaths | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  const rawTokens = tokenizeSpotlightInput(trimmed);
  const tokens = rawTokens.filter((t) => !PATH_SEPARATOR_TOKENS.has(t.toLowerCase()));
  if (tokens.length === 0 || tokens.length > 2) return null;
  if (!tokens.every(looksLikePathToken)) return null;

  const left = tokens[0];
  const right = tokens[1] ?? "";
  const inferredKind = inferKind(right ? [left, right] : [left]);
  return { left, right, inferredKind };
}

/**
 * Matches saved sessions by name or id for Spotlight search results.
 */
export function matchSavedSessions<T extends { readonly id: string; readonly name?: string | null }>(
  sessions: readonly T[],
  query: string,
  limit = 4,
): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return sessions
    .filter((s) => {
      const label = (s.name ?? s.id).toLowerCase();
      return label.includes(q) || s.id.toLowerCase().includes(q);
    })
    .slice(0, limit);
}
