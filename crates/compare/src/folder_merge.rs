//! Three-way folder merge: plan building and applying.
//!
//! Documented choices for rare situations:
//!
//! * Planning is read-only apart from the walk. The three roots must
//!   exist and be directories (checked like the folder walk checks
//!   its roots); anything else is [`Error::Missing`]. All three trees
//!   are walked from one explicit stack using `symlink_metadata`
//!   only, so symlinks are never followed, never walked and never
//!   opened, and symlink cycles cannot loop the walk.
//! * A child whose metadata fails (a race where it vanished) keeps
//!   its place in the union as a plain node, mirroring the folder
//!   walk's lenient fallback; any read of it later fails softly into
//!   `skipped`. A directory that cannot be listed mid-walk is
//!   recorded in `skipped` with the error message and its subtree is
//!   not planned; the roots themselves are hard planning errors.
//! * Content decisions are byte-based (streamed equality), never
//!   mtime-based. Text detection is strict: every side present and
//!   readable must decode as UTF-8. A symlink or directory side is
//!   never read, so it counts as binary; a file larger than
//!   [`MERGE_MAX_BYTES`] is treated as binary rather than read whole.
//! * Symlinks: an equality check involving a symlink side is false
//!   (its target is never opened), so a symlink is conservatively
//!   "changed from base". A step that would copy a symlink source —
//!   including the left copy of a binary conflict — becomes a skip
//!   with "symlink; not followed", while a conflict situation stays
//!   recorded in `conflicts`.
//! * A relative path that names a directory on the left or the
//!   right becomes one `MakeDir` step (so empty directories
//!   survive), even across a kind mismatch with a file on the other
//!   side; the mismatched file content is not merged.
//! * Applying never panics and never aborts the run: the output
//!   precondition is the only hard error, every step's failure is
//!   recorded and the run continues. Step paths are re-checked
//!   against the roots (anything that could escape them is "unsafe
//!   path"), and copy sources are re-probed with `symlink_metadata`
//!   before any bytes move.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use autocompare_diff::merge3;

use crate::binary::files_equal;
use crate::{Error, Result, TreeMergeAction, TreeMergePlan, TreeMergeReport, TreeMergeStep};

/// Files whose readable sides exceed this size are merged as binary
/// conflicts instead of being read whole into memory, keeping the
/// crate's "huge files are handled with limits" promise.
const MERGE_MAX_BYTES: u64 = 32 * 1024 * 1024;

/// Which of the three trees a walked node belongs to.
#[derive(Clone, Copy)]
enum Side {
    Base,
    Left,
    Right,
}

/// One node of one tree, as the walk saw it. A symlink is a plain
/// node with `is_symlink` set: its target is never opened.
struct SideNode {
    is_dir: bool,
    is_symlink: bool,
    /// Byte size from the walk; 0 for directories.
    size: u64,
    path: PathBuf,
}

/// The three walked trees, keyed by '/'-separated relative path.
struct Trees {
    base: BTreeMap<String, SideNode>,
    left: BTreeMap<String, SideNode>,
    right: BTreeMap<String, SideNode>,
}

/// How one file entry of the union resolved.
enum Resolved {
    /// Emit this step.
    Step(TreeMergeStep),
    /// Emit this step and record the path as a conflict.
    StepConflict(TreeMergeStep),
    /// Omit the file, recording this reason.
    Skip(String),
    /// Omit the file with this reason and record a conflict.
    SkipConflict(String),
}

/// Plans a three-way merge of two trees over their base. See the
/// crate documentation for the resolution rules.
pub(crate) fn plan_tree_merge(base: &Path, left: &Path, right: &Path) -> Result<TreeMergePlan> {
    plan_tree_merge_cancellable(base, left, right, None)
}

/// Progress callback for tree merge: `(current_index, total_steps, current_rel_path)`.
pub type TreeMergeProgressCallback<'a> = &'a dyn Fn(usize, usize, &str);

/// Plans a three-way merge with cancellation support.
pub fn plan_tree_merge_cancellable(
    base: &Path,
    left: &Path,
    right: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<TreeMergePlan> {
    require_dir(base)?;
    require_dir(left)?;
    require_dir(right)?;

    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }

    let (trees, walk_skips) = collect_trees(base, left, right)?;
    let mut plan = TreeMergePlan {
        skipped: walk_skips,
        ..TreeMergePlan::default()
    };

    let rels: BTreeSet<&str> = trees
        .base
        .keys()
        .chain(trees.left.keys())
        .chain(trees.right.keys())
        .map(|key| key.as_str())
        .collect();

    // The union is processed in sorted order, so steps and conflicts
    // come out sorted, and a directory's strict-path prefix sorts
    // before its children.
    for rel in rels.iter().copied() {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        let b = trees.base.get(rel);
        let l = trees.left.get(rel);
        let r = trees.right.get(rel);
        if is_dir_node(l) || is_dir_node(r) {
            plan.steps.push(TreeMergeStep {
                rel_path: rel.to_owned(),
                action: TreeMergeAction::MakeDir,
                merged: None,
            });
            continue;
        }
        match resolve_file(rel, b, l, r) {
            Ok(Resolved::Step(step)) => plan.steps.push(step),
            Ok(Resolved::StepConflict(step)) => {
                plan.steps.push(step);
                plan.conflicts.push(rel.to_owned());
            }
            Ok(Resolved::Skip(reason)) => plan.skipped.push((rel.to_owned(), reason)),
            Ok(Resolved::SkipConflict(reason)) => {
                plan.skipped.push((rel.to_owned(), reason));
                plan.conflicts.push(rel.to_owned());
            }
            Err(e) => plan.skipped.push((rel.to_owned(), e.to_string())),
        }
    }

    // Only the rare walk-level skips can be out of order.
    plan.skipped.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(plan)
}

/// Applies a merge plan. See the crate documentation for the
/// guarantees.
pub(crate) fn apply_tree_merge(
    left: &Path,
    right: &Path,
    plan: &TreeMergePlan,
    output: &Path,
) -> Result<TreeMergeReport> {
    apply_tree_merge_cancellable(left, right, plan, output, None, None)
}

/// Applies a merge plan with cancellation and bounded progress reporting support.
pub fn apply_tree_merge_cancellable(
    left: &Path,
    right: &Path,
    plan: &TreeMergePlan,
    output: &Path,
    cancel: Option<&AtomicBool>,
    progress: Option<TreeMergeProgressCallback<'_>>,
) -> Result<TreeMergeReport> {
    let mut report = TreeMergeReport::default();
    prepare_output(output)?;
    let total = plan.steps.len();

    for (idx, step) in plan.steps.iter().enumerate() {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }

        if let Some(p) = progress {
            p(idx, total, &step.rel_path);
        }

        let Some(target) = under(output, &step.rel_path) else {
            fail(&mut report, step, "unsafe path");
            continue;
        };
        match step.action {
            TreeMergeAction::MakeDir => apply_dir(&target, step, &mut report),
            TreeMergeAction::TakeLeft => apply_copy(left, &target, step, &mut report),
            TreeMergeAction::TakeRight => apply_copy(right, &target, step, &mut report),
            TreeMergeAction::MergedText => {
                apply_write(step.merged.as_deref(), &target, step, &mut report);
            }
            TreeMergeAction::Conflict => match step.merged.as_deref() {
                Some(lines) => apply_write(Some(lines), &target, step, &mut report),
                None => apply_copy(left, &target, step, &mut report),
            },
        }
    }

    if let Some(p) = progress {
        p(total, total, "complete");
    }

    Ok(report)
}

/// Walks all three trees from one explicit stack (symlink-safe,
/// `symlink_metadata` only) into per-tree maps of relative path to
/// node. Returns the maps plus the walk-level skips (directory
/// listings that failed mid-walk), which never abort the plan.
fn collect_trees(base: &Path, left: &Path, right: &Path) -> Result<(Trees, Vec<(String, String)>)> {
    let mut trees = Trees {
        base: BTreeMap::new(),
        left: BTreeMap::new(),
        right: BTreeMap::new(),
    };
    let mut skips: Vec<(String, String)> = Vec::new();
    let mut stack: Vec<(Side, String, PathBuf)> = vec![
        (Side::Base, String::new(), base.to_path_buf()),
        (Side::Left, String::new(), left.to_path_buf()),
        (Side::Right, String::new(), right.to_path_buf()),
    ];

    while let Some((side, rel, dir)) = stack.pop() {
        let children = match std::fs::read_dir(&dir) {
            Ok(children) => children,
            Err(e) => {
                // The roots were just validated, so failing to list
                // one is a root-planning failure; deeper failures
                // skip the subtree instead of aborting the plan.
                if rel.is_empty() {
                    return Err(Error::Io {
                        path: dir.display().to_string(),
                        source: e,
                    });
                }
                skips.push((rel, format!("reading {} failed: {}", dir.display(), e)));
                continue;
            }
        };
        for entry in children {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    skips.push((
                        rel.clone(),
                        format!("reading {} failed: {}", dir.display(), e),
                    ));
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let child_rel = join_rel(&rel, &name);
            let path = entry.path();
            let node = match std::fs::symlink_metadata(&path) {
                Ok(meta) => SideNode {
                    is_dir: meta.is_dir(),
                    is_symlink: meta.file_type().is_symlink(),
                    size: if meta.is_dir() { 0 } else { meta.len() },
                    path: path.clone(),
                },
                // A vanished child (a race) keeps its place in the
                // union as a plain node, like the folder walk's
                // lenient fallback; later reads record the skip.
                Err(_) => SideNode {
                    is_dir: false,
                    is_symlink: false,
                    size: 0,
                    path: path.clone(),
                },
            };
            if node.is_dir {
                stack.push((side, child_rel.clone(), path.clone()));
            }
            tree_mut(&mut trees, side).insert(child_rel, node);
        }
    }
    Ok((trees, skips))
}

/// The map of one side's tree.
fn tree_mut(trees: &mut Trees, side: Side) -> &mut BTreeMap<String, SideNode> {
    match side {
        Side::Base => &mut trees.base,
        Side::Left => &mut trees.left,
        Side::Right => &mut trees.right,
    }
}

/// Whether a present node is a directory.
fn is_dir_node(node: Option<&SideNode>) -> bool {
    node.is_some_and(|n| n.is_dir)
}

/// Resolves one file entry of the union by the crate's rule table.
/// `b`/`l`/`r` are the base, left and right nodes where present; at
/// least one is, by construction. Read failures surface as errors
/// and become skips in the caller; nothing here panics.
fn resolve_file(
    rel: &str,
    b: Option<&SideNode>,
    l: Option<&SideNode>,
    r: Option<&SideNode>,
) -> Result<Resolved> {
    match (b, l, r) {
        (Some(bn), Some(ln), Some(rn)) => {
            // All three present: identical sides take the left copy,
            // a one-sided change takes the changed side.
            if equal(ln, rn)? {
                return Ok(Resolved::Step(step(rel, TreeMergeAction::TakeLeft, None)));
            }
            if equal(ln, bn)? {
                return Ok(copy_step(rel, rn, TreeMergeAction::TakeRight, false));
            }
            if equal(rn, bn)? {
                return Ok(copy_step(rel, ln, TreeMergeAction::TakeLeft, false));
            }
            // Both sides changed differently.
            Ok(match merge_all3(bn, ln, rn)? {
                Some(merged) => merged_outcome(rel, merged),
                None => copy_step(rel, ln, TreeMergeAction::Conflict, true),
            })
        }
        (Some(bn), Some(ln), None) => {
            // The right side deleted the file.
            if equal(ln, bn)? {
                Ok(Resolved::Skip(
                    "deleted on the right; left unchanged from base".to_owned(),
                ))
            } else {
                // Left changed while right deleted: keep the change,
                // record the delete-vs-change conflict.
                Ok(copy_step(rel, ln, TreeMergeAction::TakeLeft, true))
            }
        }
        (Some(bn), None, Some(rn)) => {
            // The left side deleted the file.
            if equal(rn, bn)? {
                Ok(Resolved::Skip(
                    "deleted on the left; right unchanged from base".to_owned(),
                ))
            } else {
                Ok(copy_step(rel, rn, TreeMergeAction::TakeRight, true))
            }
        }
        (Some(_), None, None) => Ok(Resolved::Skip("deleted on both sides".to_owned())),
        (None, Some(ln), Some(rn)) => {
            // Both sides added the file.
            if equal(ln, rn)? {
                return Ok(Resolved::Step(step(rel, TreeMergeAction::TakeLeft, None)));
            }
            Ok(match merge_over_empty_base(ln, rn)? {
                Some(merged) => merged_outcome(rel, merged),
                None => copy_step(rel, ln, TreeMergeAction::Conflict, true),
            })
        }
        (None, Some(ln), None) => Ok(copy_step(rel, ln, TreeMergeAction::TakeLeft, false)),
        (None, None, Some(rn)) => Ok(copy_step(rel, rn, TreeMergeAction::TakeRight, false)),
        // The union only holds keys present in at least one tree, so
        // this arm cannot fire by construction; it stays a skip
        // rather than an unreachable panic.
        (None, None, None) => Ok(Resolved::Skip("not present in any tree".to_owned())),
    }
}

/// Builds a step that writes one side's copy (`TakeLeft`/`TakeRight`,
/// or the left copy of a binary `Conflict`). A symlink source is
/// never read or copied: the step becomes a skip with "symlink; not
/// followed" instead, keeping the conflict record when the situation
/// itself was a conflict. The flag comes from the walk's
/// `symlink_metadata`.
fn copy_step(rel: &str, source: &SideNode, action: TreeMergeAction, conflict: bool) -> Resolved {
    if source.is_symlink {
        return match conflict {
            true => Resolved::SkipConflict("symlink; not followed".to_owned()),
            false => Resolved::Skip("symlink; not followed".to_owned()),
        };
    }
    let made = step(rel, action, None);
    if conflict {
        Resolved::StepConflict(made)
    } else {
        Resolved::Step(made)
    }
}

/// Turns merged lines into a step: clean output becomes `MergedText`,
/// surviving conflict markers become a `Conflict` carrying them.
fn merged_outcome(rel: &str, merged: Vec<String>) -> Resolved {
    if has_markers(&merged) {
        Resolved::StepConflict(step(rel, TreeMergeAction::Conflict, Some(merged)))
    } else {
        Resolved::Step(step(rel, TreeMergeAction::MergedText, Some(merged)))
    }
}

/// One planned write.
fn step(rel: &str, action: TreeMergeAction, merged: Option<Vec<String>>) -> TreeMergeStep {
    TreeMergeStep {
        rel_path: rel.to_owned(),
        action,
        merged,
    }
}

/// Streamed byte equality of two nodes. A symlink or directory side
/// is never opened: the comparison is conservatively false, not an
/// error. Read errors propagate to the caller, which records them as
/// skips.
fn equal(a: &SideNode, b: &SideNode) -> Result<bool> {
    if !readable(a) || !readable(b) {
        return Ok(false);
    }
    files_equal(&a.path, &b.path)
}

/// Whether a node's bytes may be read: never for symlinks (their
/// target is off-limits) or directories (a kind mismatch).
fn readable(node: &SideNode) -> bool {
    !node.is_dir && !node.is_symlink
}

/// The three-way text merge when all three sides exist: `Some` merged
/// lines when every side is readable, within the size cap and valid
/// UTF-8; `None` when the file must be treated as a binary conflict
/// instead. Read errors propagate.
fn merge_all3(b: &SideNode, l: &SideNode, r: &SideNode) -> Result<Option<Vec<String>>> {
    if !readable(b) || !readable(l) || !readable(r) {
        return Ok(None);
    }
    if b.size > MERGE_MAX_BYTES || l.size > MERGE_MAX_BYTES || r.size > MERGE_MAX_BYTES {
        return Ok(None);
    }
    let base = read_text(&b.path)?;
    let left = read_text(&l.path)?;
    let right = read_text(&r.path)?;
    Ok(match (base, left, right) {
        (Some(base), Some(left), Some(right)) => Some(merge_lines(&base, &left, &right)),
        _ => None,
    })
}

/// The same merge when only the left and right sides exist: the base
/// is an empty line sequence.
fn merge_over_empty_base(l: &SideNode, r: &SideNode) -> Result<Option<Vec<String>>> {
    if !readable(l) || !readable(r) {
        return Ok(None);
    }
    if l.size > MERGE_MAX_BYTES || r.size > MERGE_MAX_BYTES {
        return Ok(None);
    }
    let left = read_text(&l.path)?;
    let right = read_text(&r.path)?;
    Ok(match (left, right) {
        (Some(left), Some(right)) => Some(merge_lines(&[], &left, &right)),
        _ => None,
    })
}

/// Runs the pure three-way merge over three line sequences.
fn merge_lines(base: &[String], left: &[String], right: &[String]) -> Vec<String> {
    let b: Vec<&str> = base.iter().map(|line| line.as_str()).collect();
    let l: Vec<&str> = left.iter().map(|line| line.as_str()).collect();
    let r: Vec<&str> = right.iter().map(|line| line.as_str()).collect();
    merge3(&b, &l, &r)
}

/// Reads a file and decodes it strictly: `Some` lines when the bytes
/// are valid UTF-8, `None` for binary content. Read errors
/// propagate; the caller records them as skips.
fn read_text(path: &Path) -> Result<Option<Vec<String>>> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    match std::str::from_utf8(&bytes) {
        Ok(text) => Ok(Some(split_lines(text))),
        Err(_) => Ok(None),
    }
}

/// Splits decoded text into lines, mirroring the text module's
/// display semantics: one trailing `\r` per line is stripped and a
/// trailing `\n` never produces a phantom empty last line.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = text.split('\n').map(strip_cr).collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// Strips one trailing `\r` (CRLF display normalisation).
fn strip_cr(line: &str) -> String {
    match line.strip_suffix('\r') {
        Some(stripped) => stripped.to_owned(),
        None => line.to_owned(),
    }
}

/// Whether merged output still carries conflict markers.
fn has_markers(merged: &[String]) -> bool {
    merged.iter().any(|line| line.starts_with("<<<<<<<"))
}

/// Joins a child name onto a relative path with '/' separators.
fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// Creates one directory (with its parents), counting it or
/// recording the failure.
fn apply_dir(target: &Path, step: &TreeMergeStep, report: &mut TreeMergeReport) {
    match std::fs::create_dir_all(target) {
        Ok(()) => report.dirs_created = report.dirs_created.saturating_add(1),
        Err(e) => fail(report, step, &e.to_string()),
    }
}

/// Copies one side's file: refuses unsafe paths, never copies through
/// a symlink, creates parent directories on demand and records
/// failures without stopping the run.
fn apply_copy(
    source_root: &Path,
    target: &Path,
    step: &TreeMergeStep,
    report: &mut TreeMergeReport,
) {
    let Some(source) = under(source_root, &step.rel_path) else {
        fail(report, step, "unsafe path");
        return;
    };
    match std::fs::symlink_metadata(&source) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fail(report, step, "symlink; not followed");
            return;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fail(report, step, "source vanished");
            return;
        }
        Err(e) => {
            fail(report, step, &e.to_string());
            return;
        }
        Ok(_) => {}
    }
    if let Err(e) = create_parent(target) {
        fail(report, step, &e);
        return;
    }
    match std::fs::copy(&source, target) {
        Ok(_) => report.written = report.written.saturating_add(1),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fail(report, step, "source vanished");
        }
        Err(e) => fail(report, step, &e.to_string()),
    }
}

/// Writes merged (or marker-laden) lines: joined with `\n` plus a
/// trailing `\n` when non-empty, creating parent directories on
/// demand. A merge step without lines is a malformed plan and fails
/// loudly instead of writing a silent empty file.
fn apply_write(
    lines: Option<&[String]>,
    target: &Path,
    step: &TreeMergeStep,
    report: &mut TreeMergeReport,
) {
    let Some(lines) = lines else {
        fail(report, step, "missing merged content");
        return;
    };
    let mut content = lines.join("\n");
    if !lines.is_empty() {
        content.push('\n');
    }
    if let Err(e) = create_parent(target) {
        fail(report, step, &e);
        return;
    }
    match std::fs::write(target, content) {
        Ok(()) => report.written = report.written.saturating_add(1),
        Err(e) => fail(report, step, &e.to_string()),
    }
}

/// Creates the target's parent directories on demand; the error is
/// returned pre-formatted for the report.
fn create_parent(target: &Path) -> std::result::Result<(), String> {
    match target.parent() {
        Some(parent) => std::fs::create_dir_all(parent).map_err(|e| e.to_string()),
        None => Err("unsafe path".to_owned()),
    }
}

/// Records one failed step with a plain-English reason.
fn fail(report: &mut TreeMergeReport, step: &TreeMergeStep, reason: &str) {
    report
        .failed
        .push((step.rel_path.clone(), reason.to_owned()));
}

/// The output precondition: a missing path is created (with its
/// parents); an existing path must be an empty directory; anything
/// else — a file, a non-empty directory — is
/// [`Error::Unsupported`]. This and I/O failures here are the only
/// hard errors of applying.
fn prepare_output(output: &Path) -> Result<()> {
    let shown = output.display().to_string();
    match std::fs::metadata(output) {
        Ok(meta) if meta.is_dir() => {
            let mut reader = std::fs::read_dir(output).map_err(|e| Error::Io {
                path: shown.clone(),
                source: e,
            })?;
            // The first entry proves the directory is not empty; an
            // entry error is an honest failure to verify emptiness.
            match reader.next() {
                Some(entry) => {
                    entry.map_err(|e| Error::Io {
                        path: shown.clone(),
                        source: e,
                    })?;
                    Err(Error::Unsupported {
                        path: shown,
                        what: "output directory exists and is not empty",
                    })
                }
                None => Ok(()),
            }
        }
        Ok(_) => Err(Error::Unsupported {
            path: shown,
            what: "output directory exists and is not empty",
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir_all(output)
            .map_err(|e| Error::Io {
                path: shown,
                source: e,
            }),
        Err(e) => Err(Error::Io {
            path: shown,
            source: e,
        }),
    }
}

/// Joins `rel` under `root`, refusing anything that could name a
/// path outside it: empty, `.` or `..` components, Windows-style
/// backslash separators, and paths whose join escapes the root (an
/// absolute `rel` replaces it). The walk only ever produces plain
/// '/'-separated names, so legitimate steps pass untouched.
fn under(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    for component in rel.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return None;
        }
        if cfg!(windows) && component.contains('\\') {
            return None;
        }
    }
    let joined = root.join(rel);
    joined.starts_with(root).then_some(joined)
}

/// Verifies that `path` exists and is a directory, mirroring the
/// root check in [`crate::folder`] (which keeps its own copy
/// private).
fn require_dir(path: &Path) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) => Err(Error::Io {
            path: path.display().to_string(),
            source: e,
        }),
    }
}

/// High-level three-way folder merge planner and executor.
#[derive(Debug, Clone)]
pub struct FolderMerge {
    base: PathBuf,
    left: PathBuf,
    right: PathBuf,
}

impl FolderMerge {
    /// Creates a new three-way folder merge over a common base directory.
    pub fn new(base: impl AsRef<Path>, left: impl AsRef<Path>, right: impl AsRef<Path>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            left: left.as_ref().to_path_buf(),
            right: right.as_ref().to_path_buf(),
        }
    }

    /// Plans a three-way merge without modifying the filesystem (read-only).
    pub fn plan(&self) -> Result<TreeMergePlan> {
        plan_tree_merge(&self.base, &self.left, &self.right)
    }

    /// Applies the merge plan into `output`.
    pub fn apply(&self, plan: &TreeMergePlan, output: impl AsRef<Path>) -> Result<TreeMergeReport> {
        apply_tree_merge(&self.left, &self.right, plan, output.as_ref())
    }

    /// Base directory root.
    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Left directory root.
    pub fn left(&self) -> &Path {
        &self.left
    }

    /// Right directory root.
    pub fn right(&self) -> &Path {
        &self.right
    }

    /// Dry run generating the merge plan (alias for `plan`).
    pub fn dry_run(&self) -> Result<TreeMergePlan> {
        self.plan()
    }

    /// Runs planning and application to `output` in one call.
    pub fn run(&self, output: impl AsRef<Path>) -> Result<TreeMergeReport> {
        let plan = self.plan()?;
        self.apply(&plan, output)
    }

    /// Runs planning and application with cancellation support.
    pub fn run_cancellable(
        &self,
        output: impl AsRef<Path>,
        cancel: &AtomicBool,
    ) -> Result<TreeMergeReport> {
        let plan = plan_tree_merge_cancellable(&self.base, &self.left, &self.right, Some(cancel))?;
        apply_tree_merge_cancellable(
            &self.left,
            &self.right,
            &plan,
            output.as_ref(),
            Some(cancel),
            None,
        )
    }

    /// Navigates to the next conflict in the plan.
    pub fn next_conflict<'a>(
        &self,
        plan: &'a TreeMergePlan,
        current: Option<&str>,
    ) -> Option<&'a str> {
        plan.next_conflict(current)
    }

    /// Navigates to the previous conflict in the plan.
    pub fn prev_conflict<'a>(
        &self,
        plan: &'a TreeMergePlan,
        current: Option<&str>,
    ) -> Option<&'a str> {
        plan.prev_conflict(current)
    }
}

impl TreeMergePlan {
    /// Whether any conflicts were detected during merge planning.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Number of conflict entries.
    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }

    /// Returns the first conflict path, if any.
    pub fn first_conflict(&self) -> Option<&str> {
        self.conflicts.first().map(|s| s.as_str())
    }

    /// Returns the last conflict path, if any.
    pub fn last_conflict(&self) -> Option<&str> {
        self.conflicts.last().map(|s| s.as_str())
    }

    /// Navigates to the next conflict relative to `current`.
    pub fn next_conflict<'a>(&'a self, current: Option<&str>) -> Option<&'a str> {
        let Some(curr) = current else {
            return self.conflicts.first().map(|s| s.as_str());
        };
        for (idx, item) in self.conflicts.iter().enumerate() {
            if item.as_str() == curr {
                return idx
                    .checked_add(1)
                    .and_then(|next_idx| self.conflicts.get(next_idx))
                    .map(|s| s.as_str());
            }
        }
        self.conflicts
            .iter()
            .find(|item| item.as_str() > curr)
            .map(|s| s.as_str())
    }

    /// Navigates to the previous conflict relative to `current`.
    pub fn prev_conflict<'a>(&'a self, current: Option<&str>) -> Option<&'a str> {
        let Some(curr) = current else {
            return self.conflicts.last().map(|s| s.as_str());
        };
        for (idx, item) in self.conflicts.iter().enumerate() {
            if item.as_str() == curr {
                return idx
                    .checked_sub(1)
                    .and_then(|prev_idx| self.conflicts.get(prev_idx))
                    .map(|s| s.as_str());
            }
        }
        self.conflicts
            .iter()
            .rev()
            .find(|item| item.as_str() < curr)
            .map(|s| s.as_str())
    }

    /// Checks if a relative path is a conflict.
    pub fn is_conflict(&self, rel_path: &str) -> bool {
        self.conflicts.iter().any(|c| c == rel_path)
    }

    /// Returns the planned step for a relative path.
    pub fn step_for(&self, rel_path: &str) -> Option<&TreeMergeStep> {
        self.steps.iter().find(|s| s.rel_path == rel_path)
    }

    /// Count of TakeLeft steps.
    pub fn take_left_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == TreeMergeAction::TakeLeft)
            .count()
    }

    /// Count of TakeRight steps.
    pub fn take_right_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == TreeMergeAction::TakeRight)
            .count()
    }

    /// Count of MergedText steps.
    pub fn merged_text_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == TreeMergeAction::MergedText)
            .count()
    }

    /// Count of Conflict steps.
    pub fn conflict_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == TreeMergeAction::Conflict)
            .count()
    }

    /// Count of MakeDir steps.
    pub fn make_dir_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == TreeMergeAction::MakeDir)
            .count()
    }

    /// Human-readable summary of the merge plan.
    pub fn summary(&self) -> String {
        format!(
            "TreeMergePlan: {} steps ({} take-L, {} take-R, {} merged, {} conflict-step, {} dirs), {} conflicts, {} skipped",
            self.steps.len(),
            self.take_left_count(),
            self.take_right_count(),
            self.merged_text_count(),
            self.conflict_step_count(),
            self.make_dir_count(),
            self.conflicts.len(),
            self.skipped.len(),
        )
    }
}

impl TreeMergeReport {
    /// Whether the merge operation succeeded with zero failed steps.
    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// Total items written or created.
    pub fn total_processed(&self) -> usize {
        self.written.saturating_add(self.dirs_created)
    }

    /// Number of failed steps.
    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }

    /// Human-readable summary of the report.
    pub fn summary(&self) -> String {
        format!(
            "TreeMergeReport: {} written, {} dirs created, {} failed",
            self.written,
            self.dirs_created,
            self.failed.len()
        )
    }
}
