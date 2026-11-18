//! Region-based three-way merge.
//!
//! Both sides are diffed against the base, each script becomes an
//! ordered list of base ranges with replacement lines, and a positional
//! walk applies non-overlapping edits cleanly. Identical edits on
//! both sides apply once. Edits from both sides whose base ranges
//! overlap (or touch, when a pure insert is involved) become one
//! conflict block with the exact marker lines `<<<<<<< left`,
//! `=======` and `>>>>>>> right`.

use crate::{DiffScript, Op, diff_lines};

/// One side's edit: base lines `[bs, be)` were replaced by `repl`
/// (empty for a pure deletion, a zero-length range for a pure insert).
struct Edit {
    bs: usize,
    be: usize,
    repl: Vec<String>,
}

/// Converts a diff script against the base into ordered edits. A
/// Delete directly followed by an Insert is one replace block.
fn script_to_edits(base_len: usize, script: &DiffScript, side: &[&str]) -> Vec<Edit> {
    let mut out = Vec::new();
    let mut p = 0usize; // base position
    let mut q = 0usize; // side position
    let mut i = 0usize;
    while let Some(op) = script.ops.get(i) {
        match op {
            Op::Equal { len } => {
                p = p.saturating_add(*len).min(base_len);
                q = q.saturating_add(*len);
            }
            Op::Delete { len } => {
                let bs = p;
                let be = p.saturating_add(*len).min(base_len);
                p = be;
                match script.ops.get(i.saturating_add(1)) {
                    Some(Op::Insert { len: ins }) => {
                        let repl = collect_lines(side, q, *ins);
                        q = q.saturating_add(*ins);
                        i += 1;
                        out.push(Edit { bs, be, repl });
                    }
                    _ => out.push(Edit {
                        bs,
                        be,
                        repl: Vec::new(),
                    }),
                }
            }
            Op::Insert { len } => {
                let repl = collect_lines(side, q, *len);
                q = q.saturating_add(*len);
                out.push(Edit { bs: p, be: p, repl });
            }
        }
        i += 1;
    }
    out
}

fn collect_lines(side: &[&str], from: usize, len: usize) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..len {
        match side.get(from.saturating_add(i)) {
            Some(line) => out.push(line.to_string()),
            None => break,
        }
    }
    out
}

/// Whether an edit `[es, ee)` joins the group `[gs, ge)`: proper
/// overlap for two non-empty ranges; touch-or-inside when a pure
/// insert (a point range) is involved.
fn joins_group(gs: usize, ge: usize, es: usize, ee: usize) -> bool {
    if gs == ge && es == ee {
        gs == es
    } else if es == ee {
        gs <= es && es <= ge
    } else if gs == ge {
        es <= gs && gs <= ee
    } else {
        es < ge && gs < ee
    }
}

/// Emits one side's version of the base range `[gs, ge)`: base lines
/// between the edits, replacement lines inside them.
fn emit_side(out: &mut Vec<String>, base: &[&str], gs: usize, ge: usize, edits: &[Edit]) {
    let mut pos = gs;
    for e in edits {
        while pos < e.bs.min(ge) {
            if let Some(line) = base.get(pos) {
                out.push(line.to_string());
            }
            pos += 1;
        }
        out.extend(e.repl.iter().cloned());
        pos = pos.max(e.be.min(ge));
    }
    while pos < ge {
        if let Some(line) = base.get(pos) {
            out.push(line.to_string());
        }
        pos += 1;
    }
}

/// Three-way merge of `base`, `left` and `right`. Never panics; the
/// result is always complete lines.
pub(crate) fn merge3(base: &[&str], left: &[&str], right: &[&str]) -> Vec<String> {
    let le = script_to_edits(base.len(), &diff_lines(base, left), left);
    let re = script_to_edits(base.len(), &diff_lines(base, right), right);
    let n = base.len();
    let mut out: Vec<String> = Vec::new();
    let (mut i, mut j, mut p) = (0usize, 0usize, 0usize);
    loop {
        if p >= n && i >= le.len() && j >= re.len() {
            break;
        }
        // Context up to the next edit (or the end of the base).
        let ls = le.get(i).map(|e| e.bs).unwrap_or(n);
        let rs = re.get(j).map(|e| e.bs).unwrap_or(n);
        let ctx_to = ls.min(rs).min(n);
        while p < ctx_to {
            if let Some(line) = base.get(p) {
                out.push(line.to_string());
            }
            p += 1;
        }
        if i >= le.len() && j >= re.len() {
            break;
        }
        // Gather the group of edits starting at `ctx_to` plus
        // everything that overlaps or touches it transitively.
        let l_start = i;
        let r_start = j;
        let (mut nl, mut nr) = (0usize, 0usize);
        let mut ge = ctx_to;
        if let Some(e) = le.get(i)
            && e.bs == ctx_to
        {
            ge = ge.max(e.be.min(n));
            i += 1;
            nl += 1;
        }
        if let Some(e) = re.get(j)
            && e.bs == ctx_to
        {
            ge = ge.max(e.be.min(n));
            j += 1;
            nr += 1;
        }
        loop {
            let mut absorbed = false;
            if let Some(e) = le.get(i)
                && joins_group(ctx_to, ge, e.bs, e.be.min(n))
            {
                ge = ge.max(e.be.min(n));
                i += 1;
                nl += 1;
                absorbed = true;
            }
            if let Some(e) = re.get(j)
                && joins_group(ctx_to, ge, e.bs, e.be.min(n))
            {
                ge = ge.max(e.be.min(n));
                j += 1;
                nr += 1;
                absorbed = true;
            }
            if !absorbed {
                break;
            }
        }
        let lslice = le.get(l_start..i).unwrap_or(&[]);
        let rslice = re.get(r_start..j).unwrap_or(&[]);
        if nl == 0 && nr == 0 {
            // Defensive: at least one edit starts at ctx_to, so this
            // cannot happen. Keep the walk progressing regardless.
            p = ctx_to;
            continue;
        }
        if nl == 0 {
            emit_side(&mut out, base, ctx_to, ge, rslice);
        } else if nr == 0 {
            emit_side(&mut out, base, ctx_to, ge, lslice);
        } else if nl == 1
            && nr == 1
            && lslice.first().is_some_and(|a| {
                rslice
                    .first()
                    .is_some_and(|b| a.bs == b.bs && a.be == b.be && a.repl == b.repl)
            })
        {
            // The same edit on both sides applies once.
            if let Some(a) = lslice.first() {
                out.extend(a.repl.iter().cloned());
            }
        } else {
            out.push("<<<<<<< left".to_string());
            emit_side(&mut out, base, ctx_to, ge, lslice);
            out.push("=======".to_string());
            emit_side(&mut out, base, ctx_to, ge, rslice);
            out.push(">>>>>>> right".to_string());
        }
        p = ge;
    }
    out
}
