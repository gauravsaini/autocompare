//! Three-way merge conflict Take resolution.
//!
//! Provides pure headless conflict resolution by taking Left, Right, Base, or Both.

#![forbid(unsafe_code)]
#![allow(dead_code)]

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
}
