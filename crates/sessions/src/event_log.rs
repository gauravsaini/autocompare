//! Append-only, event-sourced session action log with time-travel undo/redo.
//!
//! Records session path changes, option updates, folder sync steps, and 3-way
//! merge resolutions as immutable [`SessionEvent`] entries serialized in JSON
//! Lines (`.jsonl`) format. Replaying active events deterministically
//! reconstructs [`SessionReplayState`].

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::SessionError;

/// Semantic payload describing a single recorded session action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEventKind {
    /// Active comparison paths were updated.
    PathsChanged {
        /// Left input path or URI.
        left: String,
        /// Right input path or URI.
        right: String,
        /// Optional base/ancestor path for 3-way sessions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<String>,
    },
    /// A named session option was changed.
    OptionChanged {
        /// Option key (e.g., `"ignore_whitespace"`, `"keymap"`).
        key: String,
        /// Previous value prior to change.
        previous: Value,
        /// New value after change.
        next: Value,
    },
    /// A folder sync step was executed on disk.
    SyncStepApplied {
        /// Relative path affected by the sync step.
        rel_path: String,
        /// Action label (e.g., `"copy_to_right"`, `"delete_right"`).
        action: String,
        /// Optional backup location for rollback.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        backup_path: Option<String>,
    },
    /// A 3-way merge conflict was resolved.
    MergeConflictResolved {
        /// Zero-based conflict index.
        conflict_index: usize,
        /// Previous resolution choice, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous: Option<String>,
        /// Selected resolution (`"left"`, `"right"`, `"base"`, `"both"`, or `"custom"`).
        choice: String,
    },
    /// Extensible domain event for plugins and future comparators.
    CustomAction {
        /// Action identifier.
        action: String,
        /// Structured JSON payload.
        payload: Value,
    },
}

/// A single immutable record in a [`SessionEventLog`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    /// Strictly increasing 1-based sequence number within the log.
    pub seq: u64,
    /// UTC epoch timestamp in milliseconds when the event was recorded.
    pub timestamp_epoch_ms: u64,
    /// Identifier of the session owning this event.
    pub session_id: String,
    /// ODD Phase 7 trace linkage: telemetry/v1 `trace_id` (32 lowercase
    /// hex chars) joining this domain event to its execution trace.
    ///
    /// `None` when telemetry is off. `skip_serializing_if` keeps the JSON
    /// shape byte-identical to pre-trace logs whenever no trace is linked;
    /// `default` keeps old `.jsonl` lines (without the key) loadable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Event payload.
    pub kind: SessionEventKind,
}

/// Returns the canonical form of a candidate `trace_id`: `Some` when `id`
/// is exactly 32 lowercase-or-uppercase hex chars, `None` otherwise.
///
/// Malformed input normalizes to `None` so callers store no linkage rather
/// than a corrupt join key.
#[must_use]
pub fn normalize_trace_id(id: &str) -> Option<String> {
    if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(id.to_string())
    } else {
        None
    }
}

/// Deterministic projection produced by replaying active events in a [`SessionEventLog`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionReplayState {
    /// Latest left path or URI.
    pub left: String,
    /// Latest right path or URI.
    pub right: String,
    /// Latest optional base path.
    pub base: Option<String>,
    /// Active session option key-value pairs.
    pub options: BTreeMap<String, Value>,
    /// Ordered list of applied sync steps (`"action:rel_path"`).
    pub sync_applied: Vec<String>,
    /// Active 3-way merge conflict resolutions keyed by conflict index.
    pub merge_resolutions: BTreeMap<usize, String>,
    /// Number of active events folded into this state.
    pub applied_count: usize,
}

/// Append-only event log with in-memory undo/redo cursor and JSONL persistence.
#[derive(Debug, Clone)]
pub struct SessionEventLog {
    session_id: String,
    events: Vec<SessionEvent>,
    cursor: usize,
}

impl SessionEventLog {
    /// Creates an empty event log for `session_id`.
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            events: Vec::new(),
            cursor: 0,
        }
    }

    /// Returns the owning session identifier.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Appends a new event at the current cursor.
    ///
    /// If the user previously undid one or more events, the undone redo tail is
    /// truncated before appending (standard linear undo/redo semantics).
    ///
    /// Records no trace linkage (telemetry-off shape). Use
    /// [`append_with_trace`](Self::append_with_trace) to join the event to an
    /// execution trace.
    pub fn append(&mut self, kind: SessionEventKind) -> &SessionEvent {
        self.append_with_trace(kind, None)
    }

    /// Appends a new event at the current cursor, linked to `trace_id` when
    /// `Some` holds a valid 32-hex-char telemetry/v1 trace id.
    ///
    /// Malformed ids normalize to no linkage ([`normalize_trace_id`]), so
    /// the stored JSON shape stays byte-identical to [`append`](Self::append)
    /// output whenever telemetry is off or the id is unusable.
    pub fn append_with_trace(
        &mut self,
        kind: SessionEventKind,
        trace_id: Option<&str>,
    ) -> &SessionEvent {
        if self.cursor < self.events.len() {
            self.events.truncate(self.cursor);
        }
        let next_seq = self
            .events
            .last()
            .map_or(1, |e| e.seq.saturating_add(1));
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0);
        let linked = trace_id.and_then(normalize_trace_id);

        self.events.push(SessionEvent {
            seq: next_seq,
            timestamp_epoch_ms: now_ms,
            session_id: self.session_id.clone(),
            trace_id: linked,
            kind,
        });
        self.cursor = self.events.len();
        // Safe: we just pushed an element so `last()` is guaranteed Some.
        // Avoid unwrap/expect per workspace lints by returning via match.
        match self.events.last() {
            Some(ev) => ev,
            None => unreachable_fallback(&self.events),
        }
    }

    /// Returns `true` if at least one active event can be undone.
    #[must_use]
    pub const fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Returns `true` if at least one undone event can be redone.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.cursor < self.events.len()
    }

    /// Steps the cursor backward by one event, returning the undone event.
    pub fn undo(&mut self) -> Option<&SessionEvent> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor = self.cursor.saturating_sub(1);
        self.events.get(self.cursor)
    }

    /// Steps the cursor forward by one event, returning the re-applied event.
    pub fn redo(&mut self) -> Option<&SessionEvent> {
        if self.cursor >= self.events.len() {
            return None;
        }
        let idx = self.cursor;
        self.cursor = self.cursor.saturating_add(1);
        self.events.get(idx)
    }

    /// Returns the slice of currently active (non-undone) events.
    #[must_use]
    pub fn active_events(&self) -> &[SessionEvent] {
        self.events.get(..self.cursor).unwrap_or(&[])
    }

    /// Returns all recorded events including any undone redo tail.
    #[must_use]
    pub fn all_events(&self) -> &[SessionEvent] {
        &self.events
    }

    /// ODD Phase 7-remainder: joins the undo history to an execution trace.
    ///
    /// Returns every recorded event (active **and** undone redo tail) whose
    /// `trace_id` equals `trace_id`, so agents can replay what an undone
    /// operation did from its trace alone. Malformed ids match nothing;
    /// telemetry-off logs (all `trace_id: None`) never match. Query-only:
    /// cursor, replay, and serialized shapes are untouched, so behavior is
    /// identical when telemetry is disabled.
    #[must_use]
    pub fn events_for_trace(&self, trace_id: &str) -> Vec<&SessionEvent> {
        let Some(want) = normalize_trace_id(trace_id) else {
            return Vec::new();
        };
        self.events
            .iter()
            .filter(|event| event.trace_id.as_deref() == Some(want.as_str()))
            .collect()
    }

    /// Deterministically replays all active events into a [`SessionReplayState`].
    #[must_use]
    pub fn replay(&self) -> SessionReplayState {
        let mut state = SessionReplayState::default();
        for event in self.active_events() {
            match &event.kind {
                SessionEventKind::PathsChanged { left, right, base } => {
                    state.left = left.clone();
                    state.right = right.clone();
                    state.base = base.clone();
                }
                SessionEventKind::OptionChanged { key, next, .. } => {
                    state.options.insert(key.clone(), next.clone());
                }
                SessionEventKind::SyncStepApplied {
                    rel_path, action, ..
                } => {
                    state.sync_applied.push(format!("{action}:{rel_path}"));
                }
                SessionEventKind::MergeConflictResolved {
                    conflict_index,
                    choice,
                    ..
                } => {
                    state
                        .merge_resolutions
                        .insert(*conflict_index, choice.clone());
                }
                SessionEventKind::CustomAction { .. } => {}
            }
            state.applied_count = state.applied_count.saturating_add(1);
        }
        state
    }

    /// Serializes active events as newline-delimited JSON (`.jsonl`).
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Serialization`] if JSON encoding fails.
    pub fn to_jsonl(&self) -> Result<String, SessionError> {
        let mut out = String::new();
        for event in self.active_events() {
            let line = serde_json::to_string(event)?;
            out.push_str(&line);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parses a `.jsonl` event stream into a [`SessionEventLog`].
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Serialization`] if any non-empty line is invalid JSON.
    pub fn from_jsonl(session_id: impl Into<String>, text: &str) -> Result<Self, SessionError> {
        let mut events = Vec::new();
        for raw_line in text.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let ev: SessionEvent = serde_json::from_str(trimmed)?;
            events.push(ev);
        }
        let cursor = events.len();
        Ok(Self {
            session_id: session_id.into(),
            events,
            cursor,
        })
    }

    /// Atomically persists active events to a `.jsonl` file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] on serialization or filesystem write errors.
    pub fn save_jsonl(&self, path: &Path) -> Result<(), SessionError> {
        let text = self.to_jsonl()?;
        crate::atomic_write(path, text.as_bytes())
    }

    /// Loads a `.jsonl` event log from disk.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] if reading or parsing fails.
    pub fn load_jsonl(session_id: impl Into<String>, path: &Path) -> Result<Self, SessionError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_jsonl(session_id, &text)
    }
}

fn unreachable_fallback(events: &[SessionEvent]) -> &SessionEvent {
    // Only called if a Vec is empty immediately after push (impossible),
    // kept to satisfy clippy::expect_used / clippy::unreachable bans without panicking.
    static EMPTY_FALLBACK: std::sync::OnceLock<SessionEvent> = std::sync::OnceLock::new();
    let _ = events;
    EMPTY_FALLBACK.get_or_init(|| SessionEvent {
        seq: 0,
        timestamp_epoch_ms: 0,
        session_id: String::new(),
        trace_id: None,
        kind: SessionEventKind::CustomAction {
            action: "noop".to_string(),
            payload: Value::Null,
        },
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replays_appended_events_and_supports_undo_redo() {
        let mut log = SessionEventLog::new("sess-01");
        log.append(SessionEventKind::PathsChanged {
            left: "/a/left.txt".to_string(),
            right: "/b/right.txt".to_string(),
            base: None,
        });
        log.append(SessionEventKind::OptionChanged {
            key: "ignore_whitespace".to_string(),
            previous: json!(false),
            next: json!(true),
        });
        log.append(SessionEventKind::MergeConflictResolved {
            conflict_index: 0,
            previous: None,
            choice: "left".to_string(),
        });

        let full = log.replay();
        assert_eq!(full.applied_count, 3);
        assert_eq!(full.left, "/a/left.txt");
        assert_eq!(full.options.get("ignore_whitespace"), Some(&json!(true)));
        assert_eq!(full.merge_resolutions.get(&0).map(String::as_str), Some("left"));

        // Undo conflict resolution
        assert!(log.can_undo());
        let undone = log.undo().expect("undo");
        assert_eq!(undone.seq, 3);
        assert!(log.can_redo());

        let after_undo = log.replay();
        assert_eq!(after_undo.applied_count, 2);
        assert!(after_undo.merge_resolutions.is_empty());

        // Redo restores conflict resolution
        let redone = log.redo().expect("redo");
        assert_eq!(redone.seq, 3);
        assert_eq!(log.replay().merge_resolutions.get(&0).map(String::as_str), Some("left"));

        // Undo then append truncates redo tail cleanly
        log.undo();
        log.append(SessionEventKind::MergeConflictResolved {
            conflict_index: 0,
            previous: None,
            choice: "right".to_string(),
        });
        assert!(!log.can_redo());
        assert_eq!(log.replay().merge_resolutions.get(&0).map(String::as_str), Some("right"));
    }

    #[test]
    fn roundtrips_jsonl_persistence() {
        let mut log = SessionEventLog::new("sess-jsonl");
        log.append(SessionEventKind::SyncStepApplied {
            rel_path: "src/main.rs".to_string(),
            action: "copy_to_right".to_string(),
            backup_path: Some("/tmp/backup/main.rs".to_string()),
        });

        let jsonl = log.to_jsonl().expect("to_jsonl");
        let parsed = SessionEventLog::from_jsonl("sess-jsonl", &jsonl).expect("from_jsonl");
        let state = parsed.replay();
        assert_eq!(state.sync_applied, vec!["copy_to_right:src/main.rs"]);
    }

    #[test]
    fn trace_absent_when_telemetry_off_keeps_byte_identical_shape() {
        let mut log = SessionEventLog::new("sess-trace-off");
        log.append(SessionEventKind::PathsChanged {
            left: "/a/left.txt".to_string(),
            right: "/b/right.txt".to_string(),
            base: None,
        });

        let jsonl = log.to_jsonl().expect("to_jsonl");
        // No linkage key anywhere: byte-identical JSON shape to pre-trace logs.
        assert!(!jsonl.contains("trace_id"));
        let value: Value = serde_json::from_str(jsonl.trim()).expect("line json");
        assert!(value.get("trace_id").is_none());

        // Old lines (no trace_id key) still load with None linkage.
        let parsed = SessionEventLog::from_jsonl("sess-trace-off", &jsonl).expect("from_jsonl");
        assert_eq!(parsed.active_events().len(), 1);
        assert_eq!(parsed.active_events()[0].trace_id, None);
    }

    #[test]
    fn trace_id_round_trips_and_leaves_replay_untouched() {
        let trace = "0123456789abcdef0123456789abcdef";
        let mut log = SessionEventLog::new("sess-traced");
        log.append_with_trace(
            SessionEventKind::OptionChanged {
                key: "ignore_whitespace".to_string(),
                previous: json!(false),
                next: json!(true),
            },
            Some(trace),
        );

        let jsonl = log.to_jsonl().expect("to_jsonl");
        assert!(jsonl.contains(trace));

        let parsed = SessionEventLog::from_jsonl("sess-traced", &jsonl).expect("from_jsonl");
        assert_eq!(parsed.active_events().len(), 1);
        assert_eq!(
            parsed.active_events()[0].trace_id.as_deref(),
            Some(trace)
        );
        // Linkage never affects the deterministic replay projection.
        let state = parsed.replay();
        assert_eq!(state.applied_count, 1);
        assert_eq!(state.options.get("ignore_whitespace"), Some(&json!(true)));

        // Malformed ids normalize to no linkage (absent key, not null).
        assert_eq!(normalize_trace_id("zz"), None);
        assert_eq!(normalize_trace_id(""), None);
        let mut bad = SessionEventLog::new("sess-bad-trace");
        bad.append_with_trace(
            SessionEventKind::PathsChanged {
                left: "l".to_string(),
                right: "r".to_string(),
                base: None,
            },
            Some("not-a-trace-id"),
        );
        let bad_jsonl = bad.to_jsonl().expect("to_jsonl");
        assert!(!bad_jsonl.contains("trace_id"));
    }

    #[test]
    fn events_for_trace_joins_undo_history_to_execution_trace() {
        let first = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let second = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let mut log = SessionEventLog::new("sess-join");

        log.append_with_trace(
            SessionEventKind::PathsChanged {
                left: "l".to_string(),
                right: "r".to_string(),
                base: None,
            },
            Some(first),
        );
        log.append(SessionEventKind::OptionChanged {
            key: "k".to_string(),
            previous: json!(false),
            next: json!(true),
        });
        log.append_with_trace(
            SessionEventKind::MergeConflictResolved {
                conflict_index: 0,
                previous: None,
                choice: "left".to_string(),
            },
            Some(second),
        );

        // Exact join: only linked events come back, in seq order.
        let joined = log.events_for_trace(first);
        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].seq, 1);

        // Undo does not hide history from the join (redo tail included).
        log.undo();
        log.undo();
        assert_eq!(log.active_events().len(), 1);
        assert_eq!(log.events_for_trace(second).len(), 1);
        assert_eq!(log.events_for_trace(second)[0].seq, 3);

        // Telemetry-off logs and malformed ids never match.
        assert!(log.events_for_trace("not-a-trace-id").is_empty());
        assert!(log.events_for_trace("").is_empty());
        let mut plain = SessionEventLog::new("sess-plain");
        plain.append(SessionEventKind::OptionChanged {
            key: "k".to_string(),
            previous: json!(1),
            next: json!(2),
        });
        assert!(plain.events_for_trace(first).is_empty());

        // Join is query-only: cursor, replay, and JSONL shapes untouched.
        assert_eq!(log.active_events().len(), 1);
        assert_eq!(log.replay().applied_count, 1);
        let jsonl = log.to_jsonl().expect("to_jsonl");
        assert_eq!(jsonl.lines().count(), 1);
    }
}
