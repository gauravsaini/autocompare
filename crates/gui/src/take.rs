//! Three-way merge conflict Take resolution.
//!
//! Provides pure headless conflict resolution by taking Left, Right, Base, or Both.

#![forbid(unsafe_code)]

/// Choice of source when resolving a three-way merge conflict block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeChoice {
    Left,
    Right,
    Base,
    Both,
}

/// Error encountered during three-way conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeError {
    ConflictNotFound,
    MalformedConflict,
}

impl std::fmt::Display for TakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictNotFound => write!(f, "conflict block not found at specified index"),
            Self::MalformedConflict => write!(f, "malformed conflict marker structure"),
        }
    }
}

impl std::error::Error for TakeError {}

/// Resolves the N-th conflict block in a merged line stream by taking Left, Right, Base, or Both.
///
/// Scans for standard merge3 markers (`<<<<<<<`, `|||||||`, `=======`, `>>>>>>>`).
/// Replaces the entire marker block with the chosen candidate lines.
pub fn resolve_take(
    lines: &[String],
    conflict_index: usize,
    choice: TakeChoice,
) -> Result<Vec<String>, TakeError> {
    let mut out = Vec::with_capacity(lines.len());
    let mut current_conflict = 0usize;
    let mut i = 0usize;

    while i < lines.len() {
        let Some(line) = lines.get(i) else {
            break;
        };

        if line.starts_with("<<<<<<<") {
            if current_conflict == conflict_index {
                let mut left_lines = Vec::new();
                let mut base_lines = Vec::new();
                let mut right_lines = Vec::new();
                let mut section = 0; // 0 = left, 1 = base, 2 = right
                let mut found_end = false;

                i = i.saturating_add(1);
                while i < lines.len() {
                    let Some(cur) = lines.get(i) else {
                        break;
                    };
                    if cur.starts_with("|||||||") {
                        section = 1;
                    } else if cur.starts_with("=======") {
                        section = 2;
                    } else if cur.starts_with(">>>>>>>") {
                        found_end = true;
                        i = i.saturating_add(1);
                        break;
                    } else {
                        match section {
                            0 => left_lines.push(cur.clone()),
                            1 => base_lines.push(cur.clone()),
                            2 => right_lines.push(cur.clone()),
                            _ => {}
                        }
                    }
                    i = i.saturating_add(1);
                }

                if !found_end {
                    return Err(TakeError::MalformedConflict);
                }

                match choice {
                    TakeChoice::Left => out.extend(left_lines),
                    TakeChoice::Right => out.extend(right_lines),
                    TakeChoice::Base => out.extend(base_lines),
                    TakeChoice::Both => {
                        out.extend(left_lines);
                        out.extend(right_lines);
                    }
                }
                current_conflict = current_conflict.saturating_add(1);
                continue;
            } else {
                current_conflict = current_conflict.saturating_add(1);
                out.push(line.clone());
                i = i.saturating_add(1);
                continue;
            }
        }

        out.push(line.clone());
        i = i.saturating_add(1);
    }

    if conflict_index >= current_conflict {
        return Err(TakeError::ConflictNotFound);
    }

    Ok(out)
}

/// Resolves all conflict blocks in a merged line stream by taking the chosen candidate.
///
/// Replaces every conflict block (`<<<<<<<` ... `>>>>>>>`) with the chosen candidate.
pub fn resolve_take_all(lines: &[String], choice: TakeChoice) -> Result<Vec<String>, TakeError> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;

    while i < lines.len() {
        let Some(line) = lines.get(i) else {
            break;
        };

        if line.starts_with("<<<<<<<") {
            let mut left_lines = Vec::new();
            let mut base_lines = Vec::new();
            let mut right_lines = Vec::new();
            let mut section = 0; // 0 = left, 1 = base, 2 = right
            let mut found_end = false;

            i = i.saturating_add(1);
            while i < lines.len() {
                let Some(cur) = lines.get(i) else {
                    break;
                };
                if cur.starts_with("|||||||") {
                    section = 1;
                } else if cur.starts_with("=======") {
                    section = 2;
                } else if cur.starts_with(">>>>>>>") {
                    found_end = true;
                    i = i.saturating_add(1);
                    break;
                } else {
                    match section {
                        0 => left_lines.push(cur.clone()),
                        1 => base_lines.push(cur.clone()),
                        2 => right_lines.push(cur.clone()),
                        _ => {}
                    }
                }
                i = i.saturating_add(1);
            }

            if !found_end {
                return Err(TakeError::MalformedConflict);
            }

            match choice {
                TakeChoice::Left => out.extend(left_lines),
                TakeChoice::Right => out.extend(right_lines),
                TakeChoice::Base => out.extend(base_lines),
                TakeChoice::Both => {
                    out.extend(left_lines);
                    out.extend(right_lines);
                }
            }
            continue;
        }

        out.push(line.clone());
        i = i.saturating_add(1);
    }

    Ok(out)
}

/// Counts conflict blocks in a merged line stream.
pub fn conflict_count(lines: &[String]) -> usize {
    let mut count = 0usize;
    for line in lines {
        if line.starts_with("<<<<<<<") {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Returns true if the line starts with any of the standard merge3 conflict markers.
pub fn is_conflict_marker(line: &str) -> bool {
    line.starts_with("<<<<<<<")
        || line.starts_with("|||||||")
        || line.starts_with("=======")
        || line.starts_with(">>>>>>>")
}

// ----------------------------------------------------------------------------
// Take affordance geometry + conflict block height (headless).
// Geometry first: per-choice button widths + block heights; typography
// second: Mono line-height se block stack. Colors untouched.
// ----------------------------------------------------------------------------

/// Take button height px: 24 hit floor (row rhythm parity).
pub const TAKE_BUTTON_H: f32 = crate::theme::SPACE_XL;
/// Take button side pad px: do taraf section rhythm.
pub const TAKE_BUTTON_PAD_X: f32 = crate::theme::SPACE_SM;
/// Take label advance px/char: body 13 estimate (CTA precedent parity).
pub const TAKE_LABEL_ADVANCE_PX: f32 = 7.0;

/// Take button label (paint parity): hamesha "Take {Left|Right|Base|Both}".
pub fn take_choice_label(choice: TakeChoice) -> &'static str {
    match choice {
        TakeChoice::Left => "Take Left",
        TakeChoice::Right => "Take Right",
        TakeChoice::Base => "Take Base",
        TakeChoice::Both => "Take Both",
    }
}

/// Ek Take button ki width px: label chars x advance + do taraf pad.
/// Sabse lamba "Take Right" (10 chars) = 86. Pure; headless-tested.
pub fn take_choice_button_width(choice: TakeChoice) -> f32 {
    take_choice_label(choice).chars().count() as f32 * TAKE_LABEL_ADVANCE_PX
        + TAKE_BUTTON_PAD_X * 2.0
}

/// Conflict block ki height px: `line_count` Mono lines ka stack
/// (`TypoRole::Mono.line_height_px` = 15.6). Khali block par 0.
/// Pure; headless-tested.
pub fn take_conflict_block_height_px(line_count: usize) -> f32 {
    line_count as f32 * crate::theme::TypoRole::Mono.line_height_px()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_take_left_replaces_conflict_with_left_lines() {
        let lines = vec![
            "clean header".to_string(),
            "<<<<<<< left".to_string(),
            "left content".to_string(),
            "||||||| base".to_string(),
            "base content".to_string(),
            "=======".to_string(),
            "right content".to_string(),
            ">>>>>>> right".to_string(),
            "clean footer".to_string(),
        ];
        let resolved = resolve_take(&lines, 0, TakeChoice::Left).unwrap_or_default();
        assert_eq!(
            resolved,
            vec![
                "clean header".to_string(),
                "left content".to_string(),
                "clean footer".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_take_right_and_both() {
        let lines = vec![
            "<<<<<<<".to_string(),
            "L1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>>".to_string(),
        ];
        let right = resolve_take(&lines, 0, TakeChoice::Right).unwrap_or_default();
        assert_eq!(right, vec!["R1".to_string()]);

        let both = resolve_take(&lines, 0, TakeChoice::Both).unwrap_or_default();
        assert_eq!(both, vec!["L1".to_string(), "R1".to_string()]);
    }

    #[test]
    fn resolve_take_all_resolves_every_conflict_block() {
        let lines = vec![
            "h1".to_string(),
            "<<<<<<<".to_string(),
            "L1".to_string(),
            "=======".to_string(),
            "R1".to_string(),
            ">>>>>>>".to_string(),
            "mid".to_string(),
            "<<<<<<<".to_string(),
            "L2".to_string(),
            "=======".to_string(),
            "R2".to_string(),
            ">>>>>>>".to_string(),
            "end".to_string(),
        ];
        assert_eq!(conflict_count(&lines), 2);
        let res = resolve_take_all(&lines, TakeChoice::Right).unwrap_or_default();
        assert_eq!(
            res,
            vec![
                "h1".to_string(),
                "R1".to_string(),
                "mid".to_string(),
                "R2".to_string(),
                "end".to_string(),
            ]
        );
        assert_eq!(conflict_count(&res), 0);
    }

    #[test]
    fn is_conflict_marker_detects_all_four_markers() {
        assert!(is_conflict_marker("<<<<<<< left"));
        assert!(is_conflict_marker("||||||| base"));
        assert!(is_conflict_marker("======="));
        assert!(is_conflict_marker(">>>>>>> right"));
        assert!(!is_conflict_marker("normal line"));
    }

    #[test]
    fn take_buttons_share_hit_height_and_fit_labels() {
        assert_eq!(TAKE_BUTTON_H, 24.0);
        assert_eq!(take_choice_button_width(TakeChoice::Right), 86.0);
        assert_eq!(take_choice_button_width(TakeChoice::Left), 79.0);
        assert_eq!(take_choice_button_width(TakeChoice::Base), 79.0);
        assert_eq!(take_choice_button_width(TakeChoice::Both), 79.0);
        // Sab buttons ek baseline: sabse chauda Right sabko cover karta hai.
        for choice in [
            TakeChoice::Left,
            TakeChoice::Right,
            TakeChoice::Base,
            TakeChoice::Both,
        ] {
            assert!(
                take_choice_button_width(choice) <= take_choice_button_width(TakeChoice::Right)
            );
            assert!(take_choice_label(choice).chars().count() >= 9);
        }
    }

    #[test]
    fn take_conflict_block_height_stacks_mono_lines() {
        assert_eq!(take_conflict_block_height_px(0), 0.0);
        let one = take_conflict_block_height_px(1);
        assert!((one - 15.6).abs() < 0.01);
        assert!((take_conflict_block_height_px(4) - one * 4.0).abs() < 0.01);
    }
}
