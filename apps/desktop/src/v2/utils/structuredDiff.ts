export type StructuredDiffKind = "equal" | "changed" | "added" | "removed" | "typeChanged";

export interface StructuredDiffNode {
  readonly path: string;
  readonly kind: StructuredDiffKind;
  readonly leftValue: string;
  readonly rightValue: string;
}

export interface StructuredDiffOptions {
  readonly ignoreArrayOrder?: boolean;
  readonly ignoreCase?: boolean;
}

export interface ParsedStructuredResult {
  readonly ok: true;
  readonly format: "json" | "yaml";
  readonly value: unknown;
}

export interface ParsedStructuredError {
  readonly ok: false;
  readonly error: string;
}

export type ParseStructuredOutcome = ParsedStructuredResult | ParsedStructuredError;

function parseYamlScalar(raw: string): unknown {
  const trimmed = raw.trim();
  if (trimmed === "" || trimmed === "null" || trimmed === "~") return null;
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (/^-?\d+(?:\.\d+)?$/.test(trimmed)) return Number(trimmed);
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }
  if ((trimmed.startsWith("[") && trimmed.endsWith("]")) || (trimmed.startsWith("{") && trimmed.endsWith("}"))) {
    try {
      return JSON.parse(trimmed);
    } catch {
      return trimmed;
    }
  }
  return trimmed;
}

/**
 * Lightweight zero-dependency YAML subset parser supporting nested maps,
 * lists, comments, and scalars (for config files like CI/Kubernetes/YAML).
 */
function parseSimpleYaml(text: string): unknown {
  const lines = text
    .split(/\r?\n/)
    .map((line) => {
      const hashIdx = line.indexOf(" #");
      return hashIdx >= 0 ? line.slice(0, hashIdx) : line;
    })
    .filter((line) => line.trim().length > 0 && !line.trim().startsWith("#"));

  if (lines.length === 0) return {};

  interface StackFrame {
    readonly indent: number;
    readonly container: Record<string, unknown> | unknown[];
  }

  const firstTrimmed = lines[0].trim();
  const root: Record<string, unknown> | unknown[] = firstTrimmed.startsWith("- ") ? [] : {};
  const stack: StackFrame[] = [{ indent: -1, container: root }];

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const indent = line.length - line.trimStart().length;
    const trimmed = line.trim();

    while (stack.length > 1 && indent <= stack[stack.length - 1].indent) {
      stack.pop();
    }

    const parent = stack[stack.length - 1].container;

    if (trimmed.startsWith("- ")) {
      const rest = trimmed.slice(2).trim();
      if (Array.isArray(parent)) {
        const colonIdx = rest.indexOf(":");
        if (colonIdx > 0 && !rest.startsWith('"') && !rest.startsWith("'")) {
          const obj: Record<string, unknown> = {};
          const k = rest.slice(0, colonIdx).trim();
          const v = rest.slice(colonIdx + 1).trim();
          obj[k] = v === "" ? {} : parseYamlScalar(v);
          parent.push(obj);
          stack.push({ indent: indent + 1, container: obj });
        } else {
          parent.push(parseYamlScalar(rest));
        }
      }
      continue;
    }

    const colonIdx = trimmed.indexOf(":");
    if (colonIdx <= 0) continue;
    const key = trimmed.slice(0, colonIdx).trim().replace(/^["']|["']$/g, "");
    const rawVal = trimmed.slice(colonIdx + 1).trim();

    if (!Array.isArray(parent)) {
      if (rawVal === "") {
        const nextLine = lines[i + 1];
        const nextIsList = nextLine !== undefined && nextLine.trim().startsWith("- ");
        const child: Record<string, unknown> | unknown[] = nextIsList ? [] : {};
        parent[key] = child;
        stack.push({ indent, container: child });
      } else {
        parent[key] = parseYamlScalar(rawVal);
      }
    }
  }
  return root;
}

export function parseStructuredDocument(raw: string): ParseStructuredOutcome {
  const trimmed = raw.trim();
  if (!trimmed) return { ok: true, format: "json", value: {} };

  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    try {
      return { ok: true, format: "json", value: JSON.parse(trimmed) };
    } catch (e) {
      return {
        ok: false,
        error: e instanceof Error ? e.message : "Invalid JSON syntax",
      };
    }
  }

  try {
    return { ok: true, format: "json", value: JSON.parse(trimmed) };
  } catch {
    try {
      return { ok: true, format: "yaml", value: parseSimpleYaml(trimmed) };
    } catch (e) {
      return {
        ok: false,
        error: e instanceof Error ? e.message : "Could not parse JSON or YAML",
      };
    }
  }
}

export function canonicalizeValue(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(canonicalizeValue);
  }
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const k of Object.keys(value as Record<string, unknown>).sort()) {
      out[k] = canonicalizeValue((value as Record<string, unknown>)[k]);
    }
    return out;
  }
  return value;
}

export function canonicalizeJson(value: unknown): string {
  return JSON.stringify(canonicalizeValue(value), null, 2);
}

function formatLeaf(v: unknown): string {
  if (v === undefined) return "";
  if (typeof v === "string") return JSON.stringify(v);
  return JSON.stringify(v);
}

function valueType(v: unknown): string {
  if (v === null) return "null";
  if (Array.isArray(v)) return "array";
  return typeof v;
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return v !== null && typeof v === "object" && !Array.isArray(v);
}

/**
 * Computes a semantic, key-order-independent AST diff between two parsed
 * JSON/YAML documents, emitting flat JSONPath-addressed nodes.
 */
export function diffStructuredValues(
  left: unknown,
  right: unknown,
  opts: StructuredDiffOptions = {},
): StructuredDiffNode[] {
  const out: StructuredDiffNode[] = [];

  const walk = (path: string, l: unknown, r: unknown): void => {
    if (l === undefined && r !== undefined) {
      out.push({ path, kind: "added", leftValue: "", rightValue: formatLeaf(r) });
      return;
    }
    if (l !== undefined && r === undefined) {
      out.push({ path, kind: "removed", leftValue: formatLeaf(l), rightValue: "" });
      return;
    }

    const lt = valueType(l);
    const rt = valueType(r);
    if (lt !== rt) {
      out.push({
        path,
        kind: "typeChanged",
        leftValue: `${formatLeaf(l)} (${lt})`,
        rightValue: `${formatLeaf(r)} (${rt})`,
      });
      return;
    }

    if (isPlainObject(l) && isPlainObject(r)) {
      const keys = Array.from(new Set([...Object.keys(l), ...Object.keys(r)])).sort();
      if (keys.length === 0) {
        out.push({ path, kind: "equal", leftValue: "{}", rightValue: "{}" });
        return;
      }
      for (const k of keys) {
        const childPath = /^[A-Za-z_]\w*$/.test(k) ? `${path}.${k}` : `${path}[${JSON.stringify(k)}]`;
        walk(childPath, l[k], r[k]);
      }
      return;
    }

    if (Array.isArray(l) && Array.isArray(r)) {
      const arrL = opts.ignoreArrayOrder
        ? [...l].sort((a, b) => JSON.stringify(canonicalizeValue(a)).localeCompare(JSON.stringify(canonicalizeValue(b))))
        : l;
      const arrR = opts.ignoreArrayOrder
        ? [...r].sort((a, b) => JSON.stringify(canonicalizeValue(a)).localeCompare(JSON.stringify(canonicalizeValue(b))))
        : r;
      const maxLen = Math.max(arrL.length, arrR.length);
      if (maxLen === 0) {
        out.push({ path, kind: "equal", leftValue: "[]", rightValue: "[]" });
        return;
      }
      for (let i = 0; i < maxLen; i += 1) {
        walk(`${path}[${i}]`, arrL[i], arrR[i]);
      }
      return;
    }

    const eq =
      opts.ignoreCase && typeof l === "string" && typeof r === "string"
        ? l.toLowerCase() === r.toLowerCase()
        : Object.is(l, r);

    out.push({
      path,
      kind: eq ? "equal" : "changed",
      leftValue: formatLeaf(l),
      rightValue: formatLeaf(r),
    });
  };

  walk("$", left, right);
  return out;
}
