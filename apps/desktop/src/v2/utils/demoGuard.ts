/**
 * Safe demo-fallback guard for installed desktop builds.
 *
 * When a user opens a blank comparison tab (`!left && !right`), dev/mock runs
 * can load committed fixture paths, but a packaged `.app`/`.dmg` has no
 * `tools/parity-harness/fixtures/` directory on disk. Instead of surfacing a
 * red "path not found" error on first open, views treat missing demo fixtures
 * as an honest empty state prompting the user to pick two paths.
 */
export function isMissingFixtureError(err: unknown): boolean {
  const msg = (err instanceof Error ? err.message : String(err)).toLowerCase();
  return (
    msg.includes("not found") ||
    msg.includes("no such file") ||
    msg.includes("os error 2") ||
    msg.includes("missing") ||
    msg.includes("must not be empty")
  );
}
