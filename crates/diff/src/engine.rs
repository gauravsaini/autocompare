//! Myers diff engine: linear-space divide and conquer with an edit
//! budget.
//!
//! The core is the middle-snake variant of Myers' O(ND) algorithm
//! ("An O(ND) Difference Algorithm and Its Variations", variation 2):
//! bidirectional furthest-reaching path searches meet at a middle
//! snake, the problem splits at that point, and both halves are
//! diffed the same way from an explicit work stack. Memory is
//! O(n + m) per search and every search array is bounded by the edit
//! budget, never by the input length.

use std::cmp::Ordering;

use crate::Op;

/// Round cap for the cheap probe search that decides whether the
/// disjointness fallback is worth its sort.
const PROBE_ROUNDS: usize = 512;

/// Cap on the explicit work stack. Unreachable in practice (every
/// split shrinks the pending work); a breach degrades that one middle
/// to a coarse replace instead of crashing.
const MAX_STACK: usize = 1_000_000;

/// Inputs beyond this many elements degrade to a coarse script. Such
/// slice lengths cannot exist on real hardware (they would need
/// terabytes of memory), but the guard keeps the i64 coordinate math
/// provably in range.
const MAX_ELEMENTS: usize = 1 << 40;

/// Dead marker for forward points that fell outside the edit graph.
const DEAD_F: i64 = i64::MIN / 4;
/// Dead marker for reverse points that fell outside the edit graph.
const DEAD_R: i64 = i64::MAX / 4;

/// Kind tag used while building an op list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpKind {
    Equal,
    Delete,
    Insert,
}

fn kind_of(op: &Op) -> OpKind {
    match op {
        Op::Equal { .. } => OpKind::Equal,
        Op::Delete { .. } => OpKind::Delete,
        Op::Insert { .. } => OpKind::Insert,
    }
}

fn bump_len(op: &mut Op, add: usize) {
    match op {
        Op::Equal { len } => *len = len.saturating_add(add),
        Op::Delete { len } => *len = len.saturating_add(add),
        Op::Insert { len } => *len = len.saturating_add(add),
    }
}

fn make_op(kind: OpKind, len: usize) -> Op {
    match kind {
        OpKind::Equal => Op::Equal { len },
        OpKind::Delete => Op::Delete { len },
        OpKind::Insert => Op::Insert { len },
    }
}

/// Appends ops with normalization baked in: zero lengths are dropped,
/// adjacent same-kind ops merge, and replace blocks stay canonical
/// (Delete before Insert). Swapping an adjacent Delete/Insert pair is
/// always replay-safe because the two kinds consume different inputs.
struct OpBuilder {
    ops: Vec<Op>,
}

impl OpBuilder {
    fn new() -> Self {
        Self { ops: Vec::new() }
    }

    fn push(&mut self, kind: OpKind, len: usize) {
        if len == 0 {
            return;
        }
        if kind == OpKind::Delete
            && let Some(Op::Insert { len: ins }) = self.ops.last()
        {
            let ins = *ins;
            self.ops.pop();
            self.push_merged(OpKind::Delete, len);
            self.ops.push(Op::Insert { len: ins });
            return;
        }
        self.push_merged(kind, len);
    }

    /// Push without the Delete/Insert swap: the tail is never an
    /// Insert here.
    fn push_merged(&mut self, kind: OpKind, len: usize) {
        if let Some(last) = self.ops.last_mut()
            && kind_of(last) == kind
        {
            bump_len(last, len);
            return;
        }
        self.ops.push(make_op(kind, len));
    }
}

/// Diffs two slices: trim, then Myers middle-snake divide and conquer.
///
/// `budget` bounds the edit distance attempted for the top-level
/// middles; children of a split inherit a strictly smaller distance
/// bound, so the budget can only ever fire at the top. Returns the ops
/// and whether the script is exact (no fallback happened).
pub(crate) fn diff_slices<T: Ord>(a: &[T], b: &[T], budget: Option<usize>) -> (Vec<Op>, bool) {
    let mut builder = OpBuilder::new();
    if a.len() > MAX_ELEMENTS || b.len() > MAX_ELEMENTS {
        builder.push(OpKind::Delete, a.len());
        builder.push(OpKind::Insert, b.len());
        return (builder.ops, false);
    }
    let mut exact = true;
    // Pending sub-ranges with an upper bound on each range's edit
    // distance: (a_lo, a_hi, b_lo, b_hi, bound).
    let mut stack: Vec<(usize, usize, usize, usize, usize)> = Vec::new();
    stack.push((0, a.len(), 0, b.len(), budget.unwrap_or(usize::MAX)));
    while let Some((alo, ahi, blo, bhi, bound)) = stack.pop() {
        // Trim the common prefix and suffix inside this sub-range.
        let pfx = common_prefix(a, alo, ahi, b, blo, bhi);
        builder.push(OpKind::Equal, pfx);
        let ma_lo = alo.saturating_add(pfx);
        let mb_lo = blo.saturating_add(pfx);
        let sfx = common_suffix(a, ma_lo, ahi, b, mb_lo, bhi);
        let ma_hi = ahi.saturating_sub(sfx);
        let mb_hi = bhi.saturating_sub(sfx);
        let na = ma_hi.saturating_sub(ma_lo);
        let mb = mb_hi.saturating_sub(mb_lo);

        if na == 0 && mb == 0 {
            builder.push(OpKind::Equal, sfx);
            continue;
        }
        if na == 0 || mb == 0 {
            // One-sided middle: trivially minimal, no search needed.
            let dist = na.max(mb);
            if na > 0 {
                builder.push(OpKind::Delete, na);
            } else {
                builder.push(OpKind::Insert, mb);
            }
            if dist > bound {
                exact = false;
            }
            builder.push(OpKind::Equal, sfx);
            continue;
        }

        // Round cap: enough rounds to find any split of these middles,
        // and never more than the distance bound (the edit budget at
        // the top) allows.
        let size_cap = na.saturating_add(mb).saturating_add(1) / 2;
        let cap = size_cap.min(rounds_for_bound(bound, na, mb));
        // Cheap probe first: small distances (the common case) are
        // found within a few rounds, without paying for the
        // disjointness fallback below.
        let split =
            match middle_snake_split(a, ma_lo, ma_hi, b, mb_lo, mb_hi, cap.min(PROBE_ROUNDS)) {
                Some(found) => Some(found),
                None => {
                    if cap <= PROBE_ROUNDS {
                        None
                    } else if fully_disjoint(a, ma_lo, ma_hi, b, mb_lo, mb_hi) {
                        // Disjoint middles: the optimal script is exactly
                        // one Delete plus one Insert and the distance is
                        // known without any search.
                        builder.push(OpKind::Delete, na);
                        builder.push(OpKind::Insert, mb);
                        if na.saturating_add(mb) > bound {
                            exact = false;
                        }
                        builder.push(OpKind::Equal, sfx);
                        continue;
                    } else {
                        middle_snake_split(a, ma_lo, ma_hi, b, mb_lo, mb_hi, cap)
                    }
                }
            };

        match split {
            Some((x, y, dist)) => {
                if (x == 0 && y == 0) || (x == na && y == mb) {
                    // Defensive: a split point makes progress on at
                    // least one side (a d-edit path consumes at least
                    // d elements). Degrade instead of looping forever.
                    builder.push(OpKind::Delete, na);
                    builder.push(OpKind::Insert, mb);
                    builder.push(OpKind::Equal, sfx);
                    exact = false;
                    continue;
                }
                if stack.len() < MAX_STACK {
                    // Right half first (it carries the trimmed
                    // suffix), then the left half on top so it pops
                    // first. Both inherit the split distance as their
                    // bound.
                    stack.push((
                        ma_lo.saturating_add(x),
                        ahi,
                        mb_lo.saturating_add(y),
                        bhi,
                        dist,
                    ));
                    stack.push((
                        ma_lo,
                        ma_lo.saturating_add(x),
                        mb_lo,
                        mb_lo.saturating_add(y),
                        dist,
                    ));
                } else {
                    builder.push(OpKind::Delete, na);
                    builder.push(OpKind::Insert, mb);
                    builder.push(OpKind::Equal, sfx);
                    exact = false;
                }
            }
            None => {
                // Distance beyond the bound: abandon this middle with
                // a coarse but valid replace. Pending siblings keep
                // their own ranges, so the script stays replayable.
                builder.push(OpKind::Delete, na);
                builder.push(OpKind::Insert, mb);
                builder.push(OpKind::Equal, sfx);
                exact = false;
            }
        }
    }
    (builder.ops, exact)
}

/// Rounds needed so a detection proves the distance stays within
/// `bound`: an odd-delta detection at round d costs `2d - 1`, an
/// even-delta one costs `2d`.
fn rounds_for_bound(bound: usize, na: usize, mb: usize) -> usize {
    if na % 2 != mb % 2 {
        bound.saturating_add(1) / 2
    } else {
        bound / 2
    }
}

fn common_prefix<T: PartialEq>(
    a: &[T],
    alo: usize,
    ahi: usize,
    b: &[T],
    blo: usize,
    bhi: usize,
) -> usize {
    let max = ahi.saturating_sub(alo).min(bhi.saturating_sub(blo));
    let mut n = 0usize;
    while n < max {
        match (a.get(alo.saturating_add(n)), b.get(blo.saturating_add(n))) {
            (Some(x), Some(y)) if x == y => n += 1,
            _ => break,
        }
    }
    n
}

fn common_suffix<T: PartialEq>(
    a: &[T],
    alo: usize,
    ahi: usize,
    b: &[T],
    blo: usize,
    bhi: usize,
) -> usize {
    let max = ahi.saturating_sub(alo).min(bhi.saturating_sub(blo));
    let mut n = 0usize;
    while n < max {
        match (
            a.get(ahi.saturating_sub(n.saturating_add(1))),
            b.get(bhi.saturating_sub(n.saturating_add(1))),
        ) {
            (Some(x), Some(y)) if x == y => n += 1,
            _ => break,
        }
    }
    n
}

/// Whether two sub-ranges share no element at all.
fn fully_disjoint<T: Ord>(
    a: &[T],
    alo: usize,
    ahi: usize,
    b: &[T],
    blo: usize,
    bhi: usize,
) -> bool {
    let mut va: Vec<&T> = a.get(alo..ahi).unwrap_or(&[]).iter().collect();
    let mut vb: Vec<&T> = b.get(blo..bhi).unwrap_or(&[]).iter().collect();
    va.sort_unstable();
    vb.sort_unstable();
    let (mut i, mut j) = (0usize, 0usize);
    while let (Some(x), Some(y)) = (va.get(i), vb.get(j)) {
        match x.cmp(y) {
            Ordering::Equal => return false,
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
        }
    }
    true
}

/// One bidirectional furthest-reaching search over the sub-ranges
/// `a[alo..ahi]` and `b[blo..bhi]` (both non-empty), capped at `cap`
/// rounds. Returns `(x, y, dist)`: a split point relative to the
/// middles whose halves both have a smaller edit distance, plus the
/// optimal edit distance of the middles; or `None` when no overlap is
/// found within the cap.
///
/// Points that would leave the edit graph are marked dead: no valid
/// path passes through them, and dead values can never win a
/// furthest-point comparison or trigger a detection.
fn middle_snake_split<T: PartialEq>(
    a: &[T],
    alo: usize,
    ahi: usize,
    b: &[T],
    blo: usize,
    bhi: usize,
    cap: usize,
) -> Option<(usize, usize, usize)> {
    let n_i = ahi.saturating_sub(alo) as i64;
    let m_i = bhi.saturating_sub(blo) as i64;
    let delta = n_i - m_i;
    let delta_odd = delta % 2 != 0;
    let width = cap.saturating_mul(2).saturating_add(3);
    // Forward diagonals k live at index k + f_off; reverse diagonals
    // (same coordinate system) live at index k - delta + f_off.
    let f_off = cap as i64 + 1;
    let r_off = f_off - delta;
    let mut vf: Vec<i64> = vec![DEAD_F; width];
    let mut vr: Vec<i64> = vec![DEAD_R; width];
    // Sentinels read exactly once, by round 0.
    if let Some(slot) = vf.get_mut((1 + f_off) as usize) {
        *slot = 0;
    }
    if let Some(slot) = vr.get_mut((delta - 1 + r_off) as usize) {
        *slot = n_i;
    }
    for d in 0..=cap {
        let di = d as i64;
        // Forward furthest-reaching points on diagonals [-d, d].
        let mut k = -di;
        while k <= di {
            let (cdel, cins) = if k == -di {
                (
                    DEAD_F,
                    vf.get((k + 1 + f_off) as usize).copied().unwrap_or(DEAD_F),
                )
            } else if k == di {
                (
                    vf.get((k - 1 + f_off) as usize)
                        .copied()
                        .unwrap_or(DEAD_F)
                        .saturating_add(1),
                    DEAD_F,
                )
            } else {
                (
                    vf.get((k - 1 + f_off) as usize)
                        .copied()
                        .unwrap_or(DEAD_F)
                        .saturating_add(1),
                    vf.get((k + 1 + f_off) as usize).copied().unwrap_or(DEAD_F),
                )
            };
            let cdel_ok = in_graph(cdel, cdel - k, n_i, m_i);
            let cins_ok = in_graph(cins, cins - k, n_i, m_i);
            let (mut x, mut y) = if cdel_ok && (!cins_ok || cdel >= cins) {
                (cdel, cdel - k)
            } else if cins_ok {
                (cins, cins - k)
            } else {
                (DEAD_F, DEAD_F)
            };
            if !cdel_ok && !cins_ok {
                if let Some(slot) = vf.get_mut((k + f_off) as usize) {
                    *slot = DEAD_F;
                }
                k += 2;
                continue;
            }
            while x < n_i && y < m_i {
                match (
                    a.get(alo.saturating_add(x as usize)),
                    b.get(blo.saturating_add(y as usize)),
                ) {
                    (Some(u), Some(v)) if u == v => {
                        x += 1;
                        y += 1;
                    }
                    _ => break,
                }
            }
            if let Some(slot) = vf.get_mut((k + f_off) as usize) {
                *slot = x;
            }
            if delta_odd {
                // Overlap against the reverse round d - 1 values.
                let lo = delta - (di - 1);
                let hi = delta + (di - 1);
                if k >= lo && k <= hi {
                    let vrk = vr.get((k + r_off) as usize).copied().unwrap_or(DEAD_R);
                    if vrk != DEAD_R && x >= vrk {
                        return Some((x as usize, y as usize, (2 * di - 1) as usize));
                    }
                }
            }
            k += 2;
        }
        // Reverse furthest-reaching points on diagonals
        // [delta - d, delta + d].
        let mut k = delta - di;
        while k <= delta + di {
            let (cins, cdel) = if k == delta + di {
                (
                    vr.get((k - 1 + r_off) as usize).copied().unwrap_or(DEAD_R),
                    DEAD_R,
                )
            } else if k == delta - di {
                (
                    DEAD_R,
                    vr.get((k + 1 + r_off) as usize)
                        .copied()
                        .unwrap_or(DEAD_R)
                        .saturating_sub(1),
                )
            } else {
                (
                    vr.get((k - 1 + r_off) as usize).copied().unwrap_or(DEAD_R),
                    vr.get((k + 1 + r_off) as usize)
                        .copied()
                        .unwrap_or(DEAD_R)
                        .saturating_sub(1),
                )
            };
            let cins_ok = in_graph(cins, cins - k, n_i, m_i);
            let cdel_ok = in_graph(cdel, cdel - k, n_i, m_i);
            let (mut x, mut y) = if cins_ok && (!cdel_ok || cins <= cdel) {
                (cins, cins - k)
            } else if cdel_ok {
                (cdel, cdel - k)
            } else {
                (DEAD_R, DEAD_R)
            };
            if !cins_ok && !cdel_ok {
                if let Some(slot) = vr.get_mut((k + r_off) as usize) {
                    *slot = DEAD_R;
                }
                k += 2;
                continue;
            }
            while x > 0 && y > 0 {
                match (
                    a.get(alo.saturating_add((x - 1) as usize)),
                    b.get(blo.saturating_add((y - 1) as usize)),
                ) {
                    (Some(u), Some(v)) if u == v => {
                        x -= 1;
                        y -= 1;
                    }
                    _ => break,
                }
            }
            if let Some(slot) = vr.get_mut((k + r_off) as usize) {
                *slot = x;
            }
            if !delta_odd {
                // Overlap against the forward round d values.
                if k >= -di && k <= di {
                    let vfk = vf.get((k + f_off) as usize).copied().unwrap_or(DEAD_F);
                    if vfk != DEAD_F && x <= vfk {
                        return Some((x as usize, y as usize, (2 * di) as usize));
                    }
                }
            }
            k += 2;
        }
    }
    None
}

/// Whether a point is inside the edit graph.
fn in_graph(x: i64, y: i64, n: i64, m: i64) -> bool {
    x >= 0 && x <= n && y >= 0 && y <= m
}
