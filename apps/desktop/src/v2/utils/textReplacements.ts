export interface TextReplacementRule {
  readonly id: string;
  readonly left: string;
  readonly right: string;
  readonly regex: boolean;
  readonly caseSensitive: boolean;
  readonly enabled: boolean;
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
