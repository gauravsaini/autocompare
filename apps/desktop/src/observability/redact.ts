/**
 * ODD Phase 2 Leaf B — redaction gate (TS mirror of §11 default-deny rules).
 *
 * Rules mirrored from the ODD blueprint §11:
 * - File contents are NEVER emitted (only hash/size/line-count summaries,
 *   which callers attach themselves — content-bearing keys are dropped here).
 * - Absolute paths / URIs are hashed unless the `allowPaths` opt-in is set.
 * - Tokens, secrets, passwords, bearer credentials: never emitted
 *   (sensitive keys AND `key|token|secret|password`-shaped values).
 * - External-command stderr: truncated to 500 chars, then secret-scanned.
 * - Wire format is snake_case everywhere: keys are normalized on the way
 *   through (camelCase only lives at the TS view-render boundary).
 *
 * All functions are pure and never throw (safe on the emit hot path).
 */

export interface RedactOptions {
  /** Opt-in to emit raw paths (default false → hash). Mirrors `obs.allowPaths`. */
  allowPaths?: boolean;
}

/** Truncation budgets (§11). */
export const STDERR_BUDGET = 500;
const STRING_BUDGET = 2000;
const MAX_DEPTH = 10;

const REDACTED = "[redacted]";
const OMITTED_CONTENT = "[omitted:content]";
const CIRCULAR = "[circular]";
const TRUNCATED_SUFFIX = "…[truncated]";

/** Keys whose values are credentials and must never be emitted. */
const SENSITIVE_KEY_RE =
  /(token|secret|passwd|password|pwd|bearer|api[_-]?key|authorization|cookie|private[_-]?key|client[_-]?secret|access[_-]?token|refresh[_-]?token|id[_-]?token|x[_-]?token)/i;

/** Keys that carry file/text bytes rather than summaries. */
const CONTENT_KEY_RE =
  /^(content|contents|file_contents?|file_bytes|raw_bytes|raw_text|source_text|file_text|left_content|right_content|body)$/i;

/** Keys whose values are filesystem paths / URIs. */
const PATH_KEY_RE = /(path|file|dir|directory|uri|location)/i;

/** Absolute-path / URI shaped values (POSIX, Windows, file://). */
const ABS_PATH_RE = /(^\/[^ \n]*)|(^[A-Za-z]:[\\/][^ \n]*)|(^file:\/\/[^ \n]*)/;

/** `key = value` / `key: value` secret assignments inside free text. */
const SECRET_ASSIGN_RE =
  /(key|token|secret|password|passwd|pwd)(\s*[:=]\s*)([^\s,;]+)/gi;

/** Well-known token formats worth scrubbing even without a key name. */
const TOKEN_VALUE_RE =
  /-----BEGIN .*?PRIVATE KEY-----|bearer\s+[A-Za-z0-9\-._~+/=]+|gh[pousr]_[A-Za-z0-9]+|xox[bpas]-[A-Za-z0-9-]+|sk-(live|test)-[A-Za-z0-9]+|AKIA[0-9A-Z]{16}/i;

export function toSnakeCase(key: string): string {
  return key
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[-\s]+/g, "_")
    .toLowerCase();
}

/** Deterministic non-crypto hash for path placeholders (no fs/crypto deps). */
function hashPath(value: string): string {
  let h = 5381;
  for (let i = 0; i < value.length; i += 1) {
    h = ((h << 5) + h + value.charCodeAt(i)) | 0;
  }
  return `path_hash:${(h >>> 0).toString(16).padStart(8, "0")}`;
}

function truncate(value: string, budget: number): string {
  if (value.length <= budget) return value;
  return value.slice(0, budget) + TRUNCATED_SUFFIX;
}

function scrubText(value: string): string {
  if (TOKEN_VALUE_RE.test(value)) return REDACTED;
  if (SECRET_ASSIGN_RE.test(value)) {
    SECRET_ASSIGN_RE.lastIndex = 0;
    return value.replace(
      SECRET_ASSIGN_RE,
      (_m, k: string, sep: string) => `${k}${sep}${REDACTED}`,
    );
  }
  SECRET_ASSIGN_RE.lastIndex = 0;
  return value;
}

export function redactPath(value: string, opts?: RedactOptions): string {
  if (opts?.allowPaths === true) return truncate(value, STRING_BUDGET);
  return hashPath(value);
}

/** Redact stderr output: truncate to 500 chars, then secret-scan (§11). */
export function redactStderr(value: string): string {
  const cut = truncate(value, STDERR_BUDGET);
  return scrubText(cut);
}

function redactString(key: string, value: string, opts: RedactOptions): string {
  if (SENSITIVE_KEY_RE.test(key)) return REDACTED;
  if (CONTENT_KEY_RE.test(key)) return OMITTED_CONTENT;
  if (PATH_KEY_RE.test(key) || ABS_PATH_RE.test(value)) {
    return redactPath(value, opts);
  }
  return truncate(scrubText(value), STRING_BUDGET);
}

function redactValue(
  value: unknown,
  key: string,
  opts: RedactOptions,
  depth: number,
  seen: Set<object>,
): unknown {
  if (value === null || value === undefined) return value;
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (typeof value === "string") return redactString(key, value, opts);
  if (value instanceof Date) return value.toISOString();
  if (depth >= MAX_DEPTH) return REDACTED;
  if (typeof value !== "object") return REDACTED;
  if (seen.has(value as object)) return CIRCULAR;
  seen.add(value as object);
  try {
    if (Array.isArray(value)) {
      return value.map((item, i) =>
        redactValue(item, `${key}[${i}]`, opts, depth + 1, seen),
      );
    }
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      const snake = toSnakeCase(k);
      if (SENSITIVE_KEY_RE.test(snake) || SENSITIVE_KEY_RE.test(k)) {
        out[snake] = REDACTED;
        continue;
      }
      if (CONTENT_KEY_RE.test(snake) || CONTENT_KEY_RE.test(k)) {
        out[snake] = OMITTED_CONTENT;
        continue;
      }
      out[snake] = redactValue(v, snake, opts, depth + 1, seen);
    }
    return out;
  } finally {
    seen.delete(value as object);
  }
}

/**
 * Redact an attribute bag for emission: snake_case keys, no contents,
 * no tokens, no raw absolute paths (unless `allowPaths`).
 */
export function redactAttributes(
  input: Record<string, unknown>,
  opts?: RedactOptions,
): Record<string, unknown> {
  try {
    const options: RedactOptions = { allowPaths: opts?.allowPaths === true };
    const result = redactValue(input, "", options, 0, new Set());
    if (result !== null && typeof result === "object" && !Array.isArray(result)) {
      return result as Record<string, unknown>;
    }
    return {};
  } catch {
    return {};
  }
}
