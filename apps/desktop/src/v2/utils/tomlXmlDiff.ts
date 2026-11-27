/**
 * Zero-dependency TOML and XML key-path flattener & diff engine
 * used by the sample community plugin (#10: TOML / XML Config Diff).
 */

export type ConfigFormat = "toml" | "xml";

export interface ConfigDiffRow {
  readonly path: string;
  readonly kind: "same" | "changed" | "added" | "removed";
  readonly leftValue: string | null;
  readonly rightValue: string | null;
}

export interface ConfigDiffResult {
  readonly format: ConfigFormat;
  readonly rows: ConfigDiffRow[];
  readonly changedCount: number;
  readonly addedCount: number;
  readonly removedCount: number;
}

export function detectConfigFormat(text: string, hintPath?: string): ConfigFormat {
  const lower = (hintPath ?? "").toLowerCase();
  if (lower.endsWith(".xml") || lower.endsWith(".svg") || lower.endsWith(".plist")) {
    return "xml";
  }
  if (lower.endsWith(".toml")) return "toml";
  return text.trimStart().startsWith("<") ? "xml" : "toml";
}

export function parseTomlFlat(text: string): Map<string, string> {
  const out = new Map<string, string>();
  let section = "";
  for (const rawLine of text.split(/\r?\n/)) {
    const trimmed = rawLine.replace(/#.*$/, "").trim();
    if (!trimmed) continue;
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      section = trimmed.replace(/^\[+|\]+$/g, "").trim();
      continue;
    }
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    const key = trimmed.slice(0, eq).trim();
    const val = trimmed.slice(eq + 1).trim();
    const fullKey = section ? `${section}.${key}` : key;
    out.set(fullKey, val);
  }
  return out;
}

export function parseXmlFlat(text: string): Map<string, string> {
  const out = new Map<string, string>();
  const stack: string[] = [];
  const tokenRe = /<(\/?)([A-Za-z0-9_:-]+)([^>]*?)(\/?)>|([^<]+)/g;
  let match: RegExpExecArray | null;
  while ((match = tokenRe.exec(text)) !== null) {
    const [, closing, tag, attrs, selfClosing, textNode] = match;
    if (tag) {
      if (closing) {
        if (stack.length > 0) stack.pop();
        continue;
      }
      const nextPath = [...stack, tag].join("/");
      const attrRe = /([A-Za-z0-9_:-]+)\s*=\s*["']([^"']*)["']/g;
      let am: RegExpExecArray | null;
      while ((am = attrRe.exec(attrs ?? "")) !== null) {
        out.set(`${nextPath}@${am[1]}`, am[2]);
      }
      if (!selfClosing) {
        stack.push(tag);
      }
    } else if (textNode) {
      const cleaned = textNode.trim();
      if (cleaned && stack.length > 0) {
        out.set(stack.join("/"), cleaned);
      }
    }
  }
  return out;
}

export function diffTomlOrXml(
  leftText: string,
  rightText: string,
  format?: ConfigFormat,
): ConfigDiffResult {
  const resolved = format ?? detectConfigFormat(leftText);
  const leftMap = resolved === "xml" ? parseXmlFlat(leftText) : parseTomlFlat(leftText);
  const rightMap = resolved === "xml" ? parseXmlFlat(rightText) : parseTomlFlat(rightText);

  const keys = Array.from(new Set([...leftMap.keys(), ...rightMap.keys()])).sort();
  const rows: ConfigDiffRow[] = [];
  let changedCount = 0;
  let addedCount = 0;
  let removedCount = 0;

  for (const key of keys) {
    const l = leftMap.has(key) ? (leftMap.get(key) ?? null) : null;
    const r = rightMap.has(key) ? (rightMap.get(key) ?? null) : null;
    if (l === null && r !== null) {
      addedCount += 1;
      rows.push({ path: key, kind: "added", leftValue: null, rightValue: r });
    } else if (l !== null && r === null) {
      removedCount += 1;
      rows.push({ path: key, kind: "removed", leftValue: l, rightValue: null });
    } else if (l !== r) {
      changedCount += 1;
      rows.push({ path: key, kind: "changed", leftValue: l, rightValue: r });
    } else {
      rows.push({ path: key, kind: "same", leftValue: l, rightValue: r });
    }
  }

  return { format: resolved, rows, changedCount, addedCount, removedCount };
}
