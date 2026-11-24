#![forbid(unsafe_code)]

//! Three-way merge IPC commands (Wave 1c).
//!
//! Thin Tauri wrappers over the headless engine:
//! - text: [`autocompare_compare::load_lines`] for I/O plus
//!   [`autocompare_diff::merge::MergeSession`] (the same region-based
//!   3-way helper the CLI folds into `merge3`) for plan/apply.
//! - folders: [`autocompare_compare::plan_tree_merge`] /
//!   [`autocompare_compare::apply_tree_merge`].
//!
//! Registration in `commands/mod.rs` and the Tauri `invoke_handler`
//! table is owned by Wave-1R; this module only defines the handlers.
//! Destructive commands take an explicit `out_path` and refuse to run
//! without one, and never overwrite one of their inputs.

use std::path::{Path, PathBuf};

use autocompare_compare::{TreeMergePlan, TreeMergeReport};
use autocompare_diff::merge::{MergeBlock, MergeSession, TakeChoice, TakeResolution};
use serde::{Deserialize, Serialize};

/// The three text sides of a 3-way merge: (base, left, right).
type TextSides = (Vec<String>, Vec<String>, Vec<String>);

/// One conflicting hunk of a 3-way text merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextConflict {
    /// Conflict id in appearance order (matches `MergeSession` ids).
    pub id: usize,
    /// Ancestor lines for this hunk.
    pub base: Vec<String>,
    /// Left-side lines for this hunk.
    pub left: Vec<String>,
    /// Right-side lines for this hunk.
    pub right: Vec<String>,
}

/// Read-only 3-way text merge plan: conflicts only, never full file
/// bytes (front end pages content separately per the paged-fetch rule).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMergePlan {
    /// Conflicting hunks in appearance order.
    pub conflicts: Vec<TextConflict>,
    /// Same as `conflicts.len()`; duplicated for cheap front-end checks.
    pub conflict_count: usize,
    /// Number of clean (non-conflicting) blocks.
    pub clean_block_count: usize,
    /// Whether any conflict exists.
    pub has_conflicts: bool,
}

/// How to resolve one text conflict. Serde names mirror the engine
/// [`TakeChoice`] variants (`custom` carries caller-supplied lines).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextChoice {
    Base,
    Left,
    Right,
    BothLeftFirst,
    BothRightFirst,
    Custom(Vec<String>),
}

/// Resolution for a single conflict id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResolution {
    pub conflict_id: usize,
    pub choice: TextChoice,
}

/// Outcome of [`merge_text_apply`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMergeApplyReport {
    /// Where the merged text was written.
    pub out_path: String,
    /// Lines written.
    pub written_lines: usize,
    /// Conflicts found during planning.
    pub conflict_count: usize,
    /// Conflicts carrying a resolution.
    pub resolved_count: usize,
    /// Conflicts left unresolved (written back out with markers).
    pub unresolved_count: usize,
    /// Whether the output still contains conflict markers.
    pub has_conflict_markers: bool,
}

/// Outcome of [`merge_folder_apply`]: engine counts plus the
/// per-file conflict list so callers do not re-walk the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderMergeApplyReport {
    /// Where the merged tree was written.
    pub out_path: String,
    /// Files written (copies and merges).
    pub written: usize,
    /// Directories created.
    pub dirs_created: usize,
    /// Same as `conflict_paths.len()`.
    pub conflict_count: usize,
    /// Relative paths with conflicts, sorted.
    pub conflict_paths: Vec<String>,
    /// Steps that failed: (relative path, plain-English error).
    pub failed: Vec<(String, String)>,
}

/// `out_path` is mandatory for every destructive merge: `None` or a
/// blank path is a hard error, never an overwrite of an input.
fn require_out_path(out_path: Option<String>) -> Result<PathBuf, String> {
    match out_path {
        Some(raw) => {
            if raw.trim().is_empty() {
                Err(
                    "out_path is required: refusing to merge without an explicit output path"
                        .to_string(),
                )
            } else {
                Ok(PathBuf::from(raw.trim()))
            }
        }
        None => Err(
            "out_path is required: refusing to merge without an explicit output path".to_string(),
        ),
    }
}

/// True when both paths name the same file (fast path plus
/// canonicalized comparison for existing files).
fn is_same_file(candidate: &Path, existing: &Path) -> bool {
    if candidate == existing {
        return true;
    }
    match (
        std::fs::canonicalize(candidate),
        std::fs::canonicalize(existing),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Refuses an output that would overwrite one of the inputs.
fn reject_input_output(out: &Path, inputs: &[&str]) -> Result<(), String> {
    for input in inputs {
        if input.trim().is_empty() {
            continue;
        }
        let input_path = Path::new(*input);
        if is_same_file(out, input_path) {
            return Err(format!(
                "out_path '{}' must not overwrite input '{input}'",
                out.display()
            ));
        }
    }
    Ok(())
}

/// Loads three text files into line vectors (engine helper, so error
/// shapes stay identical to the CLI).
fn load_three(base: &str, left: &str, right: &str) -> Result<TextSides, String> {
    let b = autocompare_compare::load_lines(Path::new(base)).map_err(|e| e.to_string())?;
    let l = autocompare_compare::load_lines(Path::new(left)).map_err(|e| e.to_string())?;
    let r = autocompare_compare::load_lines(Path::new(right)).map_err(|e| e.to_string())?;
    Ok((b, l, r))
}

fn refs(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).collect()
}

/// Plans a 3-way text merge: conflict/hunk list with per-hunk
/// base/left/right lines. Read-only; missing paths error as
/// `path not found: ...`.
#[tauri::command]
pub fn merge_text_plan(base: String, left: String, right: String) -> Result<TextMergePlan, String> {
    let (b, l, r) = load_three(&base, &left, &right)?;
    let session = MergeSession::new(&refs(&b), &refs(&l), &refs(&r));
    let mut conflicts = Vec::new();
    let mut clean_block_count = 0usize;
    for block in session.blocks() {
        match block {
            MergeBlock::Clean(_) => {
                clean_block_count = clean_block_count.saturating_add(1);
            }
            MergeBlock::Conflict {
                id,
                base,
                left,
                right,
                ..
            } => conflicts.push(TextConflict {
                id: *id,
                base: base.clone(),
                left: left.clone(),
                right: right.clone(),
            }),
        }
    }
    let conflict_count = conflicts.len();
    Ok(TextMergePlan {
        conflicts,
        conflict_count,
        clean_block_count,
        has_conflicts: conflict_count > 0,
    })
}

/// Applies a 3-way text merge with explicit per-conflict resolutions.
///
/// Unresolved conflicts are written back out with standard markers
/// and counted in `unresolved_count`. Unknown conflict ids are a hard
/// error. `out_path` is required and must not name one of the inputs.
#[tauri::command]
pub fn merge_text_apply(
    base: String,
    left: String,
    right: String,
    resolutions: Vec<TextResolution>,
    out_path: Option<String>,
) -> Result<TextMergeApplyReport, String> {
    let out = require_out_path(out_path)?;
    reject_input_output(&out, &[&base, &left, &right])?;

    let (b, l, r) = load_three(&base, &left, &right)?;
    let mut session = MergeSession::new(&refs(&b), &refs(&l), &refs(&r));

    let mut unknown: Vec<usize> = Vec::new();
    for resolution in &resolutions {
        let applied = match &resolution.choice {
            TextChoice::Base => session.resolve_conflict(
                resolution.conflict_id,
                TakeResolution::Choice(TakeChoice::Base),
            ),
            TextChoice::Left => session.resolve_conflict(
                resolution.conflict_id,
                TakeResolution::Choice(TakeChoice::Left),
            ),
            TextChoice::Right => session.resolve_conflict(
                resolution.conflict_id,
                TakeResolution::Choice(TakeChoice::Right),
            ),
            TextChoice::BothLeftFirst => session.resolve_conflict(
                resolution.conflict_id,
                TakeResolution::Choice(TakeChoice::BothLeftFirst),
            ),
            TextChoice::BothRightFirst => session.resolve_conflict(
                resolution.conflict_id,
                TakeResolution::Choice(TakeChoice::BothRightFirst),
            ),
            TextChoice::Custom(lines) => session.resolve_conflict(
                resolution.conflict_id,
                TakeResolution::Custom(lines.clone()),
            ),
        };
        if !applied {
            unknown.push(resolution.conflict_id);
        }
    }
    if !unknown.is_empty() {
        let ids: Vec<String> = unknown.iter().map(|id| id.to_string()).collect();
        return Err(format!("unknown conflict id(s): {}", ids.join(", ")));
    }

    let conflict_count = session.conflict_count();
    let resolved_count = session.resolved_count();
    let unresolved_count = session.unresolved_count();
    let rendered = session.render_lines();
    let has_conflict_markers = rendered.iter().any(|line| line.starts_with("<<<<<<<"));

    // Same framing as the CLI `write_merged`: LF-joined with one
    // trailing newline, empty output stays empty.
    let mut text = rendered.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {} failed: {e}", parent.display()))?;
    }
    std::fs::write(&out, text).map_err(|e| format!("writing {} failed: {e}", out.display()))?;

    Ok(TextMergeApplyReport {
        out_path: out.display().to_string(),
        written_lines: rendered.len(),
        conflict_count,
        resolved_count,
        unresolved_count,
        has_conflict_markers,
    })
}

/// Plans a 3-way folder merge of two trees over their common base.
/// Read-only; missing roots error as `path not found: ...`.
#[tauri::command]
pub fn merge_folder_plan(
    base: String,
    left: String,
    right: String,
) -> Result<TreeMergePlan, String> {
    autocompare_compare::plan_tree_merge(Path::new(&base), Path::new(&left), Path::new(&right))
        .map_err(|e| e.to_string())
}

/// Applies a folder merge `plan` into `out_path`, taking copied
/// content from `left`/`right`. `out_path` is required and must not
/// be one of the input roots (the engine additionally requires the
/// output to be missing or empty).
#[tauri::command]
pub fn merge_folder_apply(
    left: String,
    right: String,
    plan: TreeMergePlan,
    out_path: Option<String>,
) -> Result<FolderMergeApplyReport, String> {
    let out = require_out_path(out_path)?;
    reject_input_output(&out, &[&left, &right])?;

    let report: TreeMergeReport =
        autocompare_compare::apply_tree_merge(Path::new(&left), Path::new(&right), &plan, &out)
            .map_err(|e| e.to_string())?;
    Ok(FolderMergeApplyReport {
        out_path: out.display().to_string(),
        written: report.written,
        dirs_created: report.dirs_created,
        conflict_count: plan.conflicts.len(),
        conflict_paths: plan.conflicts.clone(),
        failed: report.failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_dir(tag: &str) -> PathBuf {
        let n = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut dir = std::env::temp_dir();
        dir.push(format!("wave1c-merge-{}-{}-{n}", std::process::id(), tag));
        dir
    }

    fn write_file(path: &Path, content: &str) -> bool {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        std::fs::write(path, content).is_ok()
    }

    fn read_file(path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn fixture(name: &str) -> Option<String> {
        let candidate = format!("../../../tools/parity-harness/fixtures/{name}");
        if Path::new(&candidate).exists() {
            Some(candidate)
        } else {
            None
        }
    }

    fn write_text_tree(dir: &Path, base: &str, left: &str, right: &str) -> bool {
        write_file(&dir.join("base.txt"), base)
            && write_file(&dir.join("left.txt"), left)
            && write_file(&dir.join("right.txt"), right)
    }

    fn plan_paths(dir: &Path) -> (String, String, String) {
        (
            dir.join("base.txt").display().to_string(),
            dir.join("left.txt").display().to_string(),
            dir.join("right.txt").display().to_string(),
        )
    }

    #[test]
    fn text_plan_conflicting_edits_yield_one_conflict() {
        let dir = unique_dir("plan");
        assert!(write_text_tree(&dir, "a\nb\nc\n", "a\nB\nc\n", "a\nC\nc\n"));
        let (base, left, right) = plan_paths(&dir);
        let plan = match merge_text_plan(base, left, right) {
            Ok(plan) => plan,
            Err(e) => {
                assert!(e.is_empty(), "plan failed: {e}");
                return;
            }
        };
        assert!(plan.has_conflicts);
        assert_eq!(plan.conflict_count, 1);
        assert_eq!(plan.conflicts.len(), 1);
        let only = match plan.conflicts.first() {
            Some(conflict) => conflict,
            None => {
                assert!(!plan.conflicts.is_empty(), "expected one conflict");
                return;
            }
        };
        assert_eq!(only.id, 0);
        assert_eq!(only.base, vec!["b".to_string()]);
        assert_eq!(only.left, vec!["B".to_string()]);
        assert_eq!(only.right, vec!["C".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_plan_disjoint_edits_have_no_conflicts() {
        let dir = unique_dir("disjoint");
        assert!(write_text_tree(
            &dir,
            "a\nb\nc\nd\n",
            "a\nB\nc\nd\n",
            "a\nb\nc\nD\n"
        ));
        let (base, left, right) = plan_paths(&dir);
        let plan = match merge_text_plan(base, left, right) {
            Ok(plan) => plan,
            Err(e) => {
                assert!(e.is_empty(), "plan failed: {e}");
                return;
            }
        };
        assert!(!plan.has_conflicts);
        assert_eq!(plan.conflict_count, 0);
        assert!(plan.conflicts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_apply_takes_left_resolution() {
        let dir = unique_dir("apply-left");
        assert!(write_text_tree(&dir, "a\nb\nc\n", "a\nB\nc\n", "a\nC\nc\n"));
        let (base, left, right) = plan_paths(&dir);
        let out = dir.join("out.txt").display().to_string();
        let report = match merge_text_apply(
            base,
            left,
            right,
            vec![TextResolution {
                conflict_id: 0,
                choice: TextChoice::Left,
            }],
            Some(out.clone()),
        ) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "apply failed: {e}");
                return;
            }
        };
        assert_eq!(report.conflict_count, 1);
        assert_eq!(report.resolved_count, 1);
        assert_eq!(report.unresolved_count, 0);
        assert!(!report.has_conflict_markers);
        assert_eq!(report.written_lines, 3);
        assert_eq!(read_file(Path::new(&out)), Some("a\nB\nc\n".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_apply_unresolved_keeps_markers_and_counts() {
        let dir = unique_dir("apply-markers");
        assert!(write_text_tree(&dir, "a\nb\nc\n", "a\nB\nc\n", "a\nC\nc\n"));
        let (base, left, right) = plan_paths(&dir);
        let out = dir.join("out.txt").display().to_string();
        let report = match merge_text_apply(base, left, right, vec![], Some(out.clone())) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "apply failed: {e}");
                return;
            }
        };
        assert!(report.has_conflict_markers);
        assert_eq!(report.unresolved_count, 1);
        assert_eq!(report.resolved_count, 0);
        let content = match read_file(Path::new(&out)) {
            Some(content) => content,
            None => {
                assert!(Path::new(&out).exists(), "expected output file");
                return;
            }
        };
        assert!(content.contains("<<<<<<<"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_apply_unknown_conflict_id_errors() {
        let dir = unique_dir("apply-unknown");
        assert!(write_text_tree(&dir, "a\nb\nc\n", "a\nB\nc\n", "a\nC\nc\n"));
        let (base, left, right) = plan_paths(&dir);
        let out = dir.join("out.txt").display().to_string();
        let result = merge_text_apply(
            base,
            left,
            right,
            vec![TextResolution {
                conflict_id: 7,
                choice: TextChoice::Right,
            }],
            Some(out),
        );
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn destructive_ops_require_explicit_out_path() {
        let dir = unique_dir("no-out");
        assert!(write_text_tree(&dir, "a\n", "a\n", "a\n"));
        let (base, left, right) = plan_paths(&dir);
        assert!(merge_text_apply(base.clone(), left.clone(), right.clone(), vec![], None).is_err());
        assert!(merge_text_apply(base, left, right, vec![], Some("   ".to_string())).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn out_path_must_not_overwrite_inputs() {
        let dir = unique_dir("overwrite");
        assert!(write_text_tree(&dir, "a\nb\n", "a\nB\n", "a\nb\n"));
        let (base, left, right) = plan_paths(&dir);
        assert!(
            merge_text_apply(
                base.clone(),
                left.clone(),
                right.clone(),
                vec![],
                Some(left.clone())
            )
            .is_err()
        );
        assert!(merge_text_apply(base.clone(), left, right, vec![], Some(base)).is_err());
        // Inputs are untouched by the refused runs.
        assert_eq!(read_file(&dir.join("base.txt")), Some("a\nb\n".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_plan_missing_path_errors() {
        let dir = unique_dir("missing");
        let missing = dir.join("nope.txt").display().to_string();
        let result = merge_text_plan(missing, "l".to_string(), "r".to_string());
        assert!(result.is_err_and(|e| e.contains("path not found")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_plan_runs_on_parity_fixtures_when_present() {
        let (Some(base), Some(left), Some(right)) = (
            fixture("m_base.txt"),
            fixture("m_left.txt"),
            fixture("m_right.txt"),
        ) else {
            return;
        };
        match merge_text_plan(base, left, right) {
            Ok(plan) => {
                assert_eq!(plan.conflict_count, plan.conflicts.len());
                assert_eq!(plan.has_conflicts, !plan.conflicts.is_empty());
            }
            Err(e) => assert!(e.is_empty(), "fixture plan failed: {e}"),
        }
    }

    fn write_folder_tree(root: &Path, keep: &str, mod_side: &str) -> bool {
        write_file(&root.join("keep.txt"), keep) && write_file(&root.join("mod.txt"), mod_side)
    }

    #[test]
    fn folder_plan_and_apply_round_trip() {
        let dir = unique_dir("folder");
        let base = dir.join("base");
        let left = dir.join("left");
        let right = dir.join("right");
        let out = dir.join("out");
        assert!(write_folder_tree(&base, "keep\n", "v1\n"));
        assert!(write_folder_tree(&left, "keep\n", "v2\n"));
        assert!(write_folder_tree(&right, "keep\n", "v1\n"));
        let plan = match merge_folder_plan(
            base.display().to_string(),
            left.display().to_string(),
            right.display().to_string(),
        ) {
            Ok(plan) => plan,
            Err(e) => {
                assert!(e.is_empty(), "folder plan failed: {e}");
                return;
            }
        };
        assert!(plan.conflicts.is_empty());
        let report = match merge_folder_apply(
            left.display().to_string(),
            right.display().to_string(),
            plan,
            Some(out.display().to_string()),
        ) {
            Ok(report) => report,
            Err(e) => {
                assert!(e.is_empty(), "folder apply failed: {e}");
                return;
            }
        };
        assert_eq!(report.conflict_count, 0);
        assert!(report.conflict_paths.is_empty());
        assert!(report.written >= 2);
        assert_eq!(read_file(&out.join("mod.txt")), Some("v2\n".to_string()));
        assert_eq!(read_file(&out.join("keep.txt")), Some("keep\n".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_plan_reports_conflicts() {
        let dir = unique_dir("folder-conflict");
        let base = dir.join("base");
        let left = dir.join("left");
        let right = dir.join("right");
        assert!(write_folder_tree(&base, "keep\n", "v1\n"));
        assert!(write_folder_tree(&left, "keep\n", "v2\n"));
        assert!(write_folder_tree(&right, "keep\n", "v3\n"));
        let plan = match merge_folder_plan(
            base.display().to_string(),
            left.display().to_string(),
            right.display().to_string(),
        ) {
            Ok(plan) => plan,
            Err(e) => {
                assert!(e.is_empty(), "folder plan failed: {e}");
                return;
            }
        };
        assert!(!plan.conflicts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_apply_requires_out_path_and_missing_root_errors() {
        let dir = unique_dir("folder-guard");
        let left = dir.join("left");
        assert!(write_file(&left.join("a.txt"), "a\n"));
        // No output path, no run -- even with a valid plan.
        let empty = TreeMergePlan::default();
        assert!(
            merge_folder_apply(
                left.display().to_string(),
                left.display().to_string(),
                empty,
                None
            )
            .is_err()
        );
        // Missing roots fail at plan time.
        let result = merge_folder_plan(
            dir.join("base").display().to_string(),
            dir.join("left").display().to_string(),
            dir.join("right").display().to_string(),
        );
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
