export interface TextReplacementRule {
  readonly id: string;
  readonly left: string;
  readonly right: string;
  readonly regex: boolean;
  readonly caseSensitive: boolean;
  readonly enabled: boolean;
}

export interface UnimportantRule {
  readonly id: string;
  readonly name: string;
  readonly pattern: string;
  readonly regex: boolean;
  readonly caseSensitive: boolean;
  readonly enabled: boolean;
}

export const DEFAULT_UNIMPORTANT_PRESETS: readonly Omit<UnimportantRule, "id">[] = [
  {
    name: "ISO Timestamps",
    pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?(?:Z|[+-]\\d{2}:?\\d{2})?",
    regex: true,
    caseSensitive: false,
    enabled: true,
  },
  {
    name: "Line Comments (//)",
    pattern: "//.*$",
    regex: true,
    caseSensitive: false,
    enabled: true,
  },
  {
    name: "Hash Comments (#)",
    pattern: "#.*$",
    regex: true,
    caseSensitive: false,
    enabled: true,
  },
  {
    name: "UUIDs",
    pattern: "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
    regex: true,
    caseSensitive: false,
    enabled: true,
  },
];

/**
 * Strips all enabled unimportant text / grammar spans from a line.
 */
export function stripUnimportantText(
  line: string,
  rules: readonly UnimportantRule[],
): string {
  const active = rules.filter((r) => r.enabled && r.pattern.length > 0);
  if (active.length === 0) return line;
  let out = line;
  for (const rule of active) {
    if (rule.regex) {
      try {
        const re = new RegExp(rule.pattern, rule.caseSensitive ? "g" : "gi");
        out = out.replace(re, "");
      } catch {
        // Ignore invalid user regex safely
      }
    } else if (rule.caseSensitive) {
      out = out.split(rule.pattern).join("");
    } else {
      const escaped = rule.pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      out = out.replace(new RegExp(escaped, "gi"), "");
    }
  }
  return out;
}

/**
 * Checks whether `leftLine` matches `rightLine` after applying all enabled
 * Beyond Compare–style Left → Right replacement rules.
 */
export function matchesWithReplacements(
  leftLine: string,
  rightLine: string,
  rules: readonly TextReplacementRule[],
): boolean {
  const active = rules.filter((r) => r.enabled && r.left.length > 0);
  if (active.length === 0) return false;
  let transformed = leftLine;
  for (const rule of active) {
    if (rule.regex) {
      try {
        const re = new RegExp(rule.left, rule.caseSensitive ? "g" : "gi");
        transformed = transformed.replace(re, rule.right);
      } catch {
        // Ignore invalid user regex without throwing into UI
      }
    } else if (rule.caseSensitive) {
      transformed = transformed.split(rule.left).join(rule.right);
    } else {
      const escaped = rule.left.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      transformed = transformed.replace(new RegExp(escaped, "gi"), rule.right);
    }
  }
  return transformed === rightLine;
}

/**
 * Combined check for `V2TextCompare`: returns true when `leftLine` and `rightLine`
 * should fold to `equal` under active Unimportant Text rules and/or Replacements.
 */
export function matchesUnderSessionRules(
  leftLine: string,
  rightLine: string,
  unimportantRules: readonly UnimportantRule[],
  replacementRules: readonly TextReplacementRule[],
  minorEnabled: boolean,
): boolean {
  const activeUnimportant = minorEnabled
    ? unimportantRules.filter((r) => r.enabled && r.pattern.length > 0)
    : [];
  const activeReplacements = replacementRules.filter((r) => r.enabled && r.left.length > 0);
  if (activeUnimportant.length === 0 && activeReplacements.length === 0) return false;

  const cleanLeft =
    activeUnimportant.length > 0 ? stripUnimportantText(leftLine, activeUnimportant) : leftLine;
  const cleanRight =
    activeUnimportant.length > 0 ? stripUnimportantText(rightLine, activeUnimportant) : rightLine;

  if (activeUnimportant.length > 0 && cleanLeft === cleanRight) return true;
  if (activeReplacements.length > 0) {
    return matchesWithReplacements(cleanLeft, cleanRight, activeReplacements);
  }
  return false;
}
