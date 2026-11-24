//! Black-box tests for the public diff API. Replay validity first: a
//! script must reconstruct both inputs exactly, for every case,
//! including budget fallbacks. Then edges, budget behavior, minimality
//! against a brute-force distance, inline spans and the three-way
//! merge. Deterministic and fast: pseudo-random data comes from a
//! small LCG; the slow exhaustive checks are `#[ignore]`d.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used
)]

use autocompare_diff::{DiffScript, EDIT_BUDGET, InlineSpan, Op, diff_lines, inline_diff, merge3};

/// Small deterministic LCG (Knuth constants) for test data.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Replays a script over `a` and `b`, returning the reconstructions.
fn replay(a: &[&str], b: &[&str], script: &DiffScript) -> (Vec<String>, Vec<String>) {
    let mut ra: Vec<String> = Vec::new();
    let mut rb: Vec<String> = Vec::new();
    let (mut ai, mut bi) = (0usize, 0usize);
    for op in &script.ops {
        match op {
            Op::Equal { len } => {
                for i in 0..*len {
                    match (a.get(ai + i), b.get(bi + i)) {
                        (Some(l), Some(r)) => {
                            ra.push((*l).to_string());
                            rb.push((*r).to_string());
                        }
                        _ => panic!("replay underflow in Equal"),
                    }
                }
                ai += len;
                bi += len;
            }
            Op::Delete { len } => {
                for i in 0..*len {
                    match a.get(ai + i) {
                        Some(l) => ra.push((*l).to_string()),
                        None => panic!("replay underflow in Delete"),
                    }
                }
                ai += len;
            }
            Op::Insert { len } => {
                for i in 0..*len {
                    match b.get(bi + i) {
                        Some(r) => rb.push((*r).to_string()),
                        None => panic!("replay underflow in Insert"),
                    }
                }
                bi += len;
            }
        }
    }
    assert_eq!(ai, a.len(), "script must consume all of a");
    assert_eq!(bi, b.len(), "script must consume all of b");
    (ra, rb)
}

/// Diffs, replays, and asserts both reconstructions are exact.
fn check(a: &[&str], b: &[&str]) -> DiffScript {
    let script = diff_lines(a, b);
    let (ra, rb) = replay(a, b, &script);
    let want_a: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    let want_b: Vec<String> = b.iter().map(|s| s.to_string()).collect();
    assert_eq!(ra, want_a, "replay of a failed for a={a:?} b={b:?}");
    assert_eq!(rb, want_b, "replay of b failed for a={a:?} b={b:?}");
    script
}

/// Output normalization: no zero lengths, no adjacent same-kind ops,
/// and replace blocks keep the canonical Delete-before-Insert order.
fn assert_canonical(script: &DiffScript) {
    for (i, op) in script.ops.iter().enumerate() {
        let len = match op {
            Op::Equal { len } | Op::Delete { len } | Op::Insert { len } => *len,
        };
        assert!(len > 0, "zero-length op at index {i}");
        if i > 0 {
            let prev = &script.ops[i - 1];
            match (prev, op) {
                (Op::Equal { .. }, Op::Equal { .. })
                | (Op::Delete { .. }, Op::Delete { .. })
                | (Op::Insert { .. }, Op::Insert { .. }) => {
                    panic!("unmerged adjacent ops at index {i}");
                }
                (Op::Insert { .. }, Op::Delete { .. }) => {
                    panic!("Insert before Delete at index {i}");
                }
                _ => {}
            }
        }
    }
}

/// Sum of Delete and Insert lengths: the script's edit count.
fn edit_count(script: &DiffScript) -> usize {
    script
        .ops
        .iter()
        .map(|op| match op {
            Op::Equal { .. } => 0,
            Op::Delete { len } | Op::Insert { len } => *len,
        })
        .sum()
}

/// Brute-force insertion/deletion distance (equals n + m - 2 * LCS).
fn brute_distance(a: &[&str], b: &[&str]) -> usize {
    let n = a.len();
    let m = b.len();
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    let first = d.first_mut();
    if let Some(row) = first {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = j;
        }
    }
    for i in 1..=n {
        for j in 1..=m {
            let sub = d[i - 1][j - 1] + if a[i - 1] == b[j - 1] { 0 } else { 2 };
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(sub);
        }
    }
    d[n][m]
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn span(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> InlineSpan {
    InlineSpan {
        a_start,
        a_end,
        b_start,
        b_end,
    }
}

// ---------------------------------------------------------------- -----
// diff_lines: edge cases, always replayed
// ---------------------------------------------------------------- -----

#[test]
fn both_empty() {
    let script = check(&[], &[]);
    assert_eq!(script.ops, Vec::<Op>::new());
    assert!(script.exact);
}

#[test]
fn both_equal_non_empty() {
    let a = ["x", "y", "z"];
    let script = check(&a, &a);
    assert_eq!(script.ops, vec![Op::Equal { len: 3 }]);
    assert!(script.exact);
}

#[test]
fn left_empty() {
    let b = ["a", "b", "c"];
    let script = check(&[], &b);
    assert_eq!(script.ops, vec![Op::Insert { len: 3 }]);
    assert!(script.exact);
}

#[test]
fn right_empty() {
    let a = ["a", "b", "c"];
    let script = check(&a, &[]);
    assert_eq!(script.ops, vec![Op::Delete { len: 3 }]);
    assert!(script.exact);
}

#[test]
fn single_line_change() {
    let script = check(&["x"], &["y"]);
    assert_eq!(
        script.ops,
        vec![Op::Delete { len: 1 }, Op::Insert { len: 1 }]
    );
    assert!(script.exact);
    assert_canonical(&script);
}

#[test]
fn insertion_at_start_middle_end() {
    let base = ["1", "2", "3", "4", "5"];
    // Start.
    let b = ["new", "1", "2", "3", "4", "5"];
    let script = check(&base, &b);
    assert_eq!(
        script.ops,
        vec![Op::Insert { len: 1 }, Op::Equal { len: 5 }]
    );
    assert!(script.exact);
    // Middle.
    let b = ["1", "2", "new", "3", "4", "5"];
    let script = check(&base, &b);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 2 },
            Op::Insert { len: 1 },
            Op::Equal { len: 3 }
        ]
    );
    assert!(script.exact);
    // End.
    let b = ["1", "2", "3", "4", "5", "new"];
    let script = check(&base, &b);
    assert_eq!(
        script.ops,
        vec![Op::Equal { len: 5 }, Op::Insert { len: 1 }]
    );
    assert!(script.exact);
}

#[test]
fn deletion_at_start_middle_end() {
    let base = ["1", "2", "3", "4", "5"];
    // Start: base loses its first line.
    let a = ["2", "3", "4", "5"];
    let script = check(&base, &a);
    assert_eq!(
        script.ops,
        vec![Op::Delete { len: 1 }, Op::Equal { len: 4 }]
    );
    assert!(script.exact);
    // Middle.
    let a = ["1", "2", "4", "5"];
    let script = check(&base, &a);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 2 },
            Op::Delete { len: 1 },
            Op::Equal { len: 2 }
        ]
    );
    assert!(script.exact);
    // End.
    let a = ["1", "2", "3", "4"];
    let script = check(&base, &a);
    assert_eq!(
        script.ops,
        vec![Op::Equal { len: 4 }, Op::Delete { len: 1 }]
    );
    assert!(script.exact);
}

#[test]
fn interleaved_multiple_changes() {
    let a = ["1", "2", "3", "4", "5", "6", "7", "8"];
    let b = ["1", "X", "3", "4", "Y", "6", "7", "8"];
    let script = check(&a, &b);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 1 },
            Op::Delete { len: 1 },
            Op::Insert { len: 1 },
            Op::Equal { len: 2 },
            Op::Delete { len: 1 },
            Op::Insert { len: 1 },
            Op::Equal { len: 3 }
        ]
    );
    assert!(script.exact);
    assert_canonical(&script);
    assert_eq!(edit_count(&script), brute_distance(&a, &b));
}

#[test]
fn repeated_identical_lines() {
    let a = ["x", "x", "x"];
    let b = ["x", "y", "x", "x"];
    let script = check(&a, &b);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 1 },
            Op::Insert { len: 1 },
            Op::Equal { len: 2 }
        ]
    );
    assert!(script.exact);
    assert_eq!(edit_count(&script), brute_distance(&a, &b));
}

#[test]
fn change_visible_only_through_context() {
    // The differing middles share no border with anything; the equal
    // lines around them are what makes the alignment visible.
    let a = ["pre", "mid", "post"];
    let b = ["pre", "mid2", "post"];
    let script = check(&a, &b);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 1 },
            Op::Delete { len: 1 },
            Op::Insert { len: 1 },
            Op::Equal { len: 1 }
        ]
    );
    assert!(script.exact);
    assert_eq!(edit_count(&script), brute_distance(&a, &b));
}

#[test]
fn near_identical_large_inputs_use_trims_only() {
    // 10k lines differing only at the ends: the trims must reduce the
    // middles to almost nothing and the script must stay minimal.
    let mut a: Vec<String> = (0..10_000).map(|i| format!("line-{i}")).collect();
    let mut b = a.clone();
    a[0] = "changed-top".to_string();
    b[9_999] = "changed-bottom".to_string();
    let ar: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
    let br: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
    let script = check(&ar, &br);
    assert!(script.exact);
    assert_eq!(edit_count(&script), 4);
    assert_eq!(
        script.ops,
        vec![
            Op::Delete { len: 1 },
            Op::Insert { len: 1 },
            Op::Equal { len: 9_998 },
            Op::Delete { len: 1 },
            Op::Insert { len: 1 }
        ]
    );
}

// ---------------------------------------------------------------- -----
// Budget behavior
// ---------------------------------------------------------------- -----

#[test]
fn budget_fallback_far_beyond_budget() {
    // 60k vs 60k lines with nothing in common: the true distance is
    // 120k, far beyond the budget. The fallback fires before any
    // expensive search, the script is coarse but valid, and the whole
    // test completes in milliseconds.
    let a: Vec<String> = (0..60_000).map(|i| format!("left-{i}")).collect();
    let b: Vec<String> = (0..60_000).map(|i| format!("right-{i}")).collect();
    let ar: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
    let br: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
    let script = check(&ar, &br);
    assert!(!script.exact, "distance 120_000 must exceed the budget");
    assert_eq!(
        script.ops,
        vec![Op::Delete { len: 60_000 }, Op::Insert { len: 60_000 }]
    );
    assert_canonical(&script);
}

#[test]
fn budget_just_under_and_at_stay_exact() {
    // Distances EDIT_BUDGET - 1 (just under) and EDIT_BUDGET (exactly
    // at it) must stay exact; only exceeding the budget degrades.
    let half = EDIT_BUDGET / 2;
    for (n, m) in [(half - 1, half), (half, half)] {
        let a: Vec<String> = (0..n).map(|i| format!("l{i}")).collect();
        let b: Vec<String> = (0..m).map(|i| format!("r{i}")).collect();
        let ar: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
        let br: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
        let script = check(&ar, &br);
        assert!(script.exact, "distance {} must stay within budget", n + m);
        assert_eq!(
            script.ops,
            vec![Op::Delete { len: n }, Op::Insert { len: m }]
        );
        assert_canonical(&script);
    }
}

#[test]
fn budget_keeps_prefix_and_suffix_context() {
    // Over-budget middles keep the common head and tail as Equal ops.
    let head: Vec<&str> = vec!["head-1", "head-2", "head-3"];
    let tail: Vec<&str> = vec!["tail-1", "tail-2"];
    let mut a: Vec<&str> = head.clone();
    a.extend((0..60_000).map(|_| "left-x"));
    a.extend(tail.clone());
    let mut b: Vec<&str> = head.clone();
    b.extend((0..60_000).map(|_| "right-y"));
    b.extend(tail.clone());
    let script = check(&a, &b);
    assert!(!script.exact);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 3 },
            Op::Delete { len: 60_000 },
            Op::Insert { len: 60_000 },
            Op::Equal { len: 2 }
        ]
    );
}

// ---------------------------------------------------------------- -----
// Minimality vs brute force
// ---------------------------------------------------------------- -----

#[test]
fn minimal_on_pseudo_random_pairs() {
    let mut rng = Lcg::new(0x1234_5678_9abc_def0);
    let alphabet = ["a", "b", "c"];
    for _ in 0..200 {
        let n = rng.below(13) as usize;
        let m = rng.below(13) as usize;
        let a: Vec<&str> = (0..n).map(|_| alphabet[rng.below(3) as usize]).collect();
        let b: Vec<&str> = (0..m).map(|_| alphabet[rng.below(3) as usize]).collect();
        let script = check(&a, &b);
        assert_canonical(&script);
        assert!(script.exact, "small input must stay within budget");
        assert_eq!(
            edit_count(&script),
            brute_distance(&a, &b),
            "non-minimal script for a={a:?} b={b:?}"
        );
    }
}

// ---------------------------------------------------------------- -----
// inline_diff
// ---------------------------------------------------------------- -----

fn assert_spans_wellformed(a: &str, b: &str, spans: &[InlineSpan]) {
    let ca = a.chars().count();
    let cb = b.chars().count();
    for s in spans {
        assert!(s.a_start <= s.a_end, "a range inverted: {s:?}");
        assert!(s.b_start <= s.b_end, "b range inverted: {s:?}");
        assert!(s.a_end <= ca, "a range out of bounds: {s:?}");
        assert!(s.b_end <= cb, "b range out of bounds: {s:?}");
    }
    for w in 1..spans.len() {
        let (p, q) = (&spans[w - 1], &spans[w]);
        assert!(p.a_end <= q.a_start, "a spans overlap: {p:?} {q:?}");
        assert!(p.b_end <= q.b_start, "b spans overlap: {p:?} {q:?}");
    }
}

#[test]
fn inline_equal_lines() {
    assert_eq!(
        inline_diff("hello world", "hello world"),
        Vec::<InlineSpan>::new()
    );
    assert_eq!(inline_diff("", ""), Vec::<InlineSpan>::new());
    assert_eq!(inline_diff("  ", "  "), Vec::<InlineSpan>::new());
}

#[test]
fn inline_one_word_changed_mid_line() {
    let a = "the quick fox";
    let b = "the slow fox";
    let spans = inline_diff(a, b);
    assert_eq!(spans, vec![span(4, 9, 4, 8)]);
    assert_spans_wellformed(a, b, &spans);
}

#[test]
fn inline_multiple_separated_changes() {
    let a = "one two three four";
    let b = "one T two three F";
    let spans = inline_diff(a, b);
    assert_eq!(spans, vec![span(4, 4, 4, 5), span(14, 18, 16, 17)]);
    assert_spans_wellformed(a, b, &spans);
}

#[test]
fn inline_one_side_empty() {
    let spans = inline_diff("", "hello world");
    assert_eq!(spans, vec![span(0, 0, 0, 11)]);
    assert_spans_wellformed("", "hello world", &spans);
}

#[test]
fn inline_deletion_mid_line() {
    let a = "hello big world";
    let b = "hello world";
    let spans = inline_diff(a, b);
    assert_eq!(spans, vec![span(6, 9, 6, 6)]);
    assert_spans_wellformed(a, b, &spans);
}

#[test]
fn inline_insertion_mid_line() {
    let a = "hello world";
    let b = "hello big world";
    let spans = inline_diff(a, b);
    assert_eq!(spans, vec![span(6, 6, 6, 9)]);
    assert_spans_wellformed(a, b, &spans);
}

#[test]
fn inline_offsets_are_character_not_byte_offsets() {
    // "héllo" is 5 chars but 6 bytes; the token after it starts at
    // char 6, not byte 7.
    let a = "héllo wörld";
    let b = "héllo wörLD";
    let spans = inline_diff(a, b);
    // Hierarchical refinement: only `ld` -> `LD` differs, still char offsets.
    assert_eq!(spans, vec![span(9, 11, 9, 11)]);
    assert_spans_wellformed(a, b, &spans);
}

#[test]
fn inline_prefix_and_suffix_never_highlighted() {
    // Whole-line replacement: everything between the shared first and
    // last words is one span; the borders stay untouched.
    let a = "alpha middle gamma";
    let b = "alpha middle-xyz gamma";
    let spans = inline_diff(a, b);
    // Hierarchical refinement: `middle` is a shared prefix, only `-xyz` is new.
    assert_eq!(spans, vec![span(12, 12, 12, 16)]);
    assert_spans_wellformed(a, b, &spans);
}

// ---------------------------------------------------------------- -----
// merge3
// ---------------------------------------------------------------- -----

#[test]
fn merge_clean_edits_in_different_regions() {
    let base = ["one", "two", "three", "four", "five", "six"];
    let left = ["one", "TWO", "three", "four", "five", "six"];
    let right = ["one", "two", "three", "four", "FIVE", "six"];
    let out = merge3(&base, &left, &right);
    assert_eq!(
        out,
        strings(&["one", "TWO", "three", "four", "FIVE", "six"])
    );
}

#[test]
fn merge_context_untouched_between_edits() {
    let base: Vec<&str> = (0..10)
        .map(|i| match i {
            2 => "old-two",
            7 => "old-seven",
            _ => "keep",
        })
        .collect();
    // Replace base lines 2 and 7 on opposite sides.
    let left: Vec<&str> = base
        .iter()
        .enumerate()
        .map(|(i, s)| if i == 2 { "new-two" } else { s })
        .collect();
    let right: Vec<&str> = base
        .iter()
        .enumerate()
        .map(|(i, s)| if i == 7 { "new-seven" } else { s })
        .collect();
    let out = merge3(&base, &left, &right);
    // Every untouched base line appears exactly once, in order, with
    // both replacements applied.
    let expected: Vec<String> = base
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == 2 {
                "new-two".to_string()
            } else if i == 7 {
                "new-seven".to_string()
            } else {
                s.to_string()
            }
        })
        .collect();
    assert_eq!(out, expected);
    assert_eq!(out.len(), base.len());
}

#[test]
fn merge_identical_edit_applied_once() {
    let base = ["a", "b", "c"];
    let left = ["a", "X", "c"];
    let right = ["a", "X", "c"];
    let out = merge3(&base, &left, &right);
    assert_eq!(out, strings(&["a", "X", "c"]));
}

#[test]
fn merge_overlapping_different_edits_conflict() {
    let base = ["1", "2", "3", "4", "5"];
    let left = ["1", "L2", "L3", "4", "5"];
    let right = ["1", "R2", "3", "4", "5"];
    let out = merge3(&base, &left, &right);
    // Left replaced base [1,3); right replaced base [1,2): the group
    // spans base [1,3), the right side keeps base line "3" as context.
    assert_eq!(
        out,
        strings(&[
            "1",
            "<<<<<<< left",
            "L2",
            "L3",
            "=======",
            "R2",
            "3",
            ">>>>>>> right",
            "4",
            "5"
        ])
    );
    assert_eq!(
        out.iter().filter(|l| *l == "<<<<<<< left").count(),
        1,
        "one conflict block"
    );
}

#[test]
fn merge_insert_at_same_point_identical_no_conflict() {
    let base = ["a", "b"];
    let left = ["a", "X", "b"];
    let right = ["a", "X", "b"];
    let out = merge3(&base, &left, &right);
    assert_eq!(out, strings(&["a", "X", "b"]));
}

#[test]
fn merge_insert_at_same_point_different_conflict() {
    let base = ["a", "b"];
    let left = ["a", "X", "b"];
    let right = ["a", "Y", "b"];
    let out = merge3(&base, &left, &right);
    assert_eq!(
        out,
        strings(&[
            "a",
            "<<<<<<< left",
            "X",
            "=======",
            "Y",
            ">>>>>>> right",
            "b"
        ])
    );
}

#[test]
fn merge_insert_touching_delete_conflicts() {
    // A pure insert at base point 1 touches the other side's delete of
    // base [1,3): per the region rules that is a conflict.
    let base = ["a", "b", "c"];
    let left = ["a", "X", "b", "c"];
    let right = ["a", "c"];
    let out = merge3(&base, &left, &right);
    // The right side deleted base[1] ("b"), so its version of the
    // conflict region is empty; "c" follows as shared context.
    assert_eq!(
        out,
        strings(&[
            "a",
            "<<<<<<< left",
            "X",
            "b",
            "=======",
            ">>>>>>> right",
            "c"
        ])
    );
}

#[test]
fn merge_transitive_overlap_chain_one_conflict_block() {
    // Left edits [0,1) and [2,3); right edits [1,3). Left's first edit
    // does not overlap right's, but the chain through [1,3) pulls
    // both left edits into one conflict group covering base [1,3)...
    // the first left edit stays clean and only the touching region
    // conflicts.
    let base = ["b0", "b1", "b2", "b3", "b4"];
    let left = ["LA", "b1", "LC", "b3", "b4"];
    let right = ["b0", "R", "b3", "b4"];
    let out = merge3(&base, &left, &right);
    assert_eq!(
        out,
        strings(&[
            "LA",
            "<<<<<<< left",
            "b1",
            "LC",
            "=======",
            "R",
            ">>>>>>> right",
            "b3",
            "b4"
        ])
    );
}

#[test]
fn merge_base_empty() {
    // base empty, both sides equal: left's lines, no conflict.
    let left = ["x", "y"];
    let right = ["x", "y"];
    assert_eq!(merge3(&[], &left, &right), strings(&["x", "y"]));
    // base empty, sides differ: one conflict block.
    let right = ["z"];
    assert_eq!(
        merge3(&[], &left, &right),
        strings(&["<<<<<<< left", "x", "y", "=======", "z", ">>>>>>> right"])
    );
    // All three empty: nothing at all.
    assert_eq!(merge3(&[], &[], &[]), Vec::<String>::new());
}

#[test]
fn merge_both_sides_identical_files() {
    let base = ["r0", "r1", "r2"];
    let left = ["r0-mod", "r1", "r2", "r3"];
    let right = ["r0-mod", "r1", "r2", "r3"];
    assert_eq!(merge3(&base, &left, &right), strings(&left));
}

// ---------------------------------------------------------------- -----
// Exhaustive checks (slow; run with `cargo test --release -- --ignored`)
// ---------------------------------------------------------------- -----

/// All sequences over `alphabet` with length <= max_len.
fn all_sequences<'a>(alphabet: &[&'a str], max_len: usize) -> Vec<Vec<&'a str>> {
    let mut out: Vec<Vec<&str>> = vec![Vec::new()];
    let mut len_start = 0usize;
    for _ in 0..max_len {
        let end = out.len();
        for i in len_start..end {
            let base = out.get(i).cloned().unwrap_or_default();
            for item in alphabet {
                let mut seq = base.clone();
                seq.push(item);
                out.push(seq);
            }
        }
        len_start = end;
    }
    out
}

fn verify_case(a: &[&str], b: &[&str]) {
    let script = diff_lines(a, b);
    let (ra, rb) = replay(a, b, &script);
    assert_eq!(ra.len(), a.len(), "replay len a for {a:?} {b:?}");
    assert_eq!(rb.len(), b.len(), "replay len b for {a:?} {b:?}");
    for (got, want) in ra.iter().zip(a.iter()) {
        assert_eq!(got.as_str(), *want);
    }
    for (got, want) in rb.iter().zip(b.iter()) {
        assert_eq!(got.as_str(), *want);
    }
    assert!(script.exact, "in-budget input must stay exact: {a:?} {b:?}");
    assert_eq!(
        edit_count(&script),
        brute_distance(a, b),
        "non-minimal: a={a:?} b={b:?} ops={:?}",
        script.ops
    );
}

#[test]
#[ignore = "exhaustive: ~130k pairs, run in release with --ignored"]
fn exhaustive_alphabet3_len5() {
    let alphabet = ["a", "b", "c"];
    let seqs = all_sequences(&alphabet, 5);
    for a in &seqs {
        for b in &seqs {
            verify_case(a, b);
        }
    }
}

#[test]
#[ignore = "exhaustive: ~65k pairs, run in release with --ignored"]
fn exhaustive_alphabet2_len7() {
    let alphabet = ["a", "b"];
    let seqs = all_sequences(&alphabet, 7);
    for a in &seqs {
        for b in &seqs {
            verify_case(a, b);
        }
    }
}

#[test]
#[ignore = "exhaustive: ~116k pairs, run in release with --ignored"]
fn exhaustive_alphabet4_len4() {
    let alphabet = ["a", "b", "c", "d"];
    let seqs = all_sequences(&alphabet, 4);
    for a in &seqs {
        for b in &seqs {
            verify_case(a, b);
        }
    }
}

#[test]
#[ignore = "broad random sweep: 20k pairs up to length 30"]
fn random_pairs_medium() {
    let mut rng = Lcg::new(0xfeed_beef_cafe_babe);
    let alphabet = ["a", "b", "c", "d"];
    for _ in 0..20_000 {
        let n = rng.below(31) as usize;
        let m = rng.below(31) as usize;
        let a: Vec<&str> = (0..n).map(|_| alphabet[rng.below(4) as usize]).collect();
        let b: Vec<&str> = (0..m).map(|_| alphabet[rng.below(4) as usize]).collect();
        verify_case(&a, &b);
    }
}

#[test]
#[ignore = "deep random sweep: 100k pairs up to length 60 (deep split trees)"]
fn random_pairs_deep() {
    let mut rng = Lcg::new(0xdead_beef_1111_2222);
    let alphabet = ["a", "b", "c"];
    for _ in 0..100_000 {
        let n = rng.below(61) as usize;
        let m = rng.below(61) as usize;
        let a: Vec<&str> = (0..n).map(|_| alphabet[rng.below(3) as usize]).collect();
        let b: Vec<&str> = (0..m).map(|_| alphabet[rng.below(3) as usize]).collect();
        verify_case(&a, &b);
    }
}
