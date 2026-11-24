#![forbid(unsafe_code)]

//! In-memory desktop session cache.
//!
//! # Paged-fetch rule (locked for Wave 1)
//!
//! The front end MUST page large content through commands with explicit
//! `(offset, limit)` windows. `AppState` never holds full file bytes, and
//! no command may serialize a whole file over IPC -- pages only. This keeps
//! the WebView responsive for multi-megabyte inputs and bounds every IPC
//! payload regardless of file size.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Opaque session identifier shared with the front end (`SessionId` DTO).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Skeleton cache entry. Wave 1 extends this with kind-specific metadata
/// (never raw file bytes -- see the paged-fetch rule above).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub id: SessionId,
    /// Session kind label; Wave 1 locks this to the engine `SessionKind`
    /// serde names.
    pub kind: String,
    pub name: String,
    pub locked: bool,
    /// Number of fetched pages currently held for this session.
    pub page_count: u64,
}

/// Shared Tauri state (`AppState::default()` is registered via `.manage()`).
#[derive(Debug, Default)]
pub struct AppState {
    sessions: Mutex<HashMap<SessionId, CachedSession>>,
}

impl AppState {
    pub fn insert(&self, session: CachedSession) -> bool {
        match self.sessions.lock() {
            Ok(mut guard) => {
                guard.insert(session.id.clone(), session);
                true
            }
            Err(_) => false,
        }
    }

    pub fn get(&self, id: &SessionId) -> Option<CachedSession> {
        match self.sessions.lock() {
            Ok(guard) => guard.get(id).cloned(),
            Err(_) => None,
        }
    }

    pub fn remove(&self, id: &SessionId) -> bool {
        match self.sessions.lock() {
            Ok(mut guard) => guard.remove(id).is_some(),
            Err(_) => false,
        }
    }

    pub fn len(&self) -> usize {
        match self.sessions.lock() {
            Ok(guard) => guard.len(),
            Err(_) => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> CachedSession {
        CachedSession {
            id: SessionId::new(id.to_string()),
            kind: "TextCompare".to_string(),
            name: id.to_string(),
            locked: false,
            page_count: 0,
        }
    }

    #[test]
    fn insert_get_remove_round_trip() {
        let state = AppState::default();
        assert!(state.is_empty());
        assert!(state.insert(sample("a")));
        assert_eq!(state.len(), 1);
        let fetched = state.get(&SessionId::new("a".to_string()));
        assert!(fetched.is_some());
        assert!(state.remove(&SessionId::new("a".to_string())));
        assert!(state.is_empty());
    }

    #[test]
    fn missing_session_returns_none() {
        let state = AppState::default();
        assert!(state.get(&SessionId::new("nope".to_string())).is_none());
        assert!(!state.remove(&SessionId::new("nope".to_string())));
    }
}
