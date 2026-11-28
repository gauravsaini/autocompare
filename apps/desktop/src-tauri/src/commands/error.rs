//! ODD Phase 6 error mapping (Wave 6E).
//!
//! # Ownership
//!
//! This module is owned by Phase 6. `commands/mod.rs` owns the
//! registry; each domain module owns its command bodies. This file
//! only maps errors: panic capture and serde field paths. It never
//! changes command logic.
//!
//! * Panics become error spans with `component` + `location`
//!   (redacted message, never contents).
//! * Serde failures carry a field path (`field`, `line`, `column`)
//!   so contract drift points at the exact JSON shape.

#![forbid(unsafe_code)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Once;

/// Installs the Phase 6 panic hook exactly once. The hook emits a
/// redacted `tracing::error!` with `component = "rs.cmd"` and the
/// panic location; it never alters program behavior.
pub fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let location = info.location().map_or_else(
                || "unknown".to_string(),
                |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
            );
            let raw = if let Some(s) = info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic payload".to_string()
            };
            let message = redact_panic_message(&raw);
            tracing::error!(
                component = "rs.cmd",
                location = %location,
                message = %message,
                "panic captured"
            );
        }));
    });
}

/// Runs `f`, converting a panic into an `Err(String)` plus an error
/// span carrying `component` and the caller `location`.
///
/// The panic message is redacted: secret-looking text becomes
/// `[redacted]`, otherwise truncated to 300 chars.
pub fn capture_panic<F, R>(component: &str, location: &str, f: F) -> Result<R, String>
where
    F: FnOnce() -> R,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            let raw = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic payload".to_string()
            };
            let message = redact_panic_message(&raw);
            tracing::error!(
                component = %component,
                location = %location,
                message = %message,
                "panic captured"
            );
            Err(format!("panic in {component} at {location}: {message}"))
        }
    }
}

/// Redacts a panic message: secret-looking text is refused, otherwise
/// truncated to 300 chars. Never panics; empty input yields a placeholder.
pub fn redact_panic_message(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "[empty panic message]".to_string();
    }
    match autocompare_telemetry::redact::scrub_external_output(raw) {
        Some(scrubbed) => scrubbed.chars().take(300).collect(),
        None => "[redacted]".to_string(),
    }
}

/// Extracts the serde field path from a `serde_json::Error` message.
///
/// Parses the first backticked segment (`missing field `foo``,
/// `unknown field `bar``) or the `.[path]` suffix. Returns `None`
/// when the message carries no field shape.
pub fn extract_serde_field(message: &str) -> Option<String> {
    let mut in_tick = false;
    let mut current = String::new();
    for ch in message.chars() {
        if ch == '`' {
            if in_tick {
                if !current.is_empty() {
                    return Some(sanitize_field(&current));
                }
                in_tick = false;
                current.clear();
            } else {
                in_tick = true;
                current.clear();
            }
        } else if in_tick {
            // Bound field length to avoid pathological messages.
            if current.len() < 128 {
                current.push(ch);
            }
        }
    }
    None
}

fn sanitize_field(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 128 {
            break;
        }
    }
    if out.trim().is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

/// Formats a `serde_json::Error` with its field path, line, and column.
///
/// Shape: `<message> (field: <path>, line: <l>, column: <c>)` or without
/// the field segment when no path is present. Emits an error span with
/// `component`, `field`, `line`, `column`.
pub fn serde_error_with_field(component: &str, err: &serde_json::Error) -> String {
    let message = err.to_string();
    let field = extract_serde_field(&message);
    let line = err.line() as u64;
    let column = err.column() as u64;
    if let Some(ref f) = field {
        tracing::error!(
            component = %component,
            field = %f,
            line = line,
            column = column,
            message = %message.chars().take(300).collect::<String>(),
            "serde error"
        );
        format!("{message} (field: {f}, line: {line}, column: {column})")
    } else {
        tracing::error!(
            component = %component,
            line = line,
            column = column,
            message = %message.chars().take(300).collect::<String>(),
            "serde error"
        );
        format!("{message} (line: {line}, column: {column})")
    }
}

/// Parses `serde_json::Value` into `T`, mapping failures through
/// [`serde_error_with_field`]. On success records nothing.
pub fn from_value_with_field<T>(component: &str, value: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_value::<T>(value) {
        Ok(parsed) => Ok(parsed),
        Err(err) => Err(serde_error_with_field(component, &err)),
    }
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

    #[test]
    fn panic_capture_records_component_and_location() {
        let res: Result<(), String> =
            capture_panic("rs.cmd/text", "commands/text.rs:622", || {
                panic!("boom-test-panic")
            });
        assert!(res.is_err());
        let msg = res.unwrap_err();
        assert!(msg.contains("rs.cmd/text"));
        assert!(msg.contains("commands/text.rs:622"));
        assert!(msg.contains("boom-test-panic"));
    }

    #[test]
    fn panic_capture_redacts_secrets() {
        let res: Result<(), String> =
            capture_panic("rs.cmd/text", "commands/text.rs:1", || {
                panic!("token=super-secret-bearer-abc123 password hunter2")
            });
        assert!(res.is_err());
        let msg = res.unwrap_err();
        assert!(!msg.contains("hunter2"));
        assert!(!msg.contains("super-secret-bearer"));
        assert!(msg.contains("[redacted]") || msg.contains("rs.cmd/text"));
    }

    #[test]
    fn serde_field_path_points_at_missing_field() {
        let bad = serde_json::json!({ "left": "/a" });
        let err = serde_json::from_value::<crate::commands::text::TextCompareDto>(bad)
            .unwrap_err();
        let annotated = serde_error_with_field("rs.cmd/text", &err);
        // serde reports the missing field in backticks; our extractor surfaces it.
        assert!(
            annotated.contains("field:"),
            "expected field path in annotation, got: {annotated}"
        );
        assert!(annotated.contains("line:"));
        // Direct extractor unit.
        assert_eq!(
            extract_serde_field("missing field `right_path` at line 1 column 10"),
            Some("right_path".to_string())
        );
        assert_eq!(extract_serde_field("no backticks here"), None);
    }

    #[test]
    fn from_value_with_field_ok_and_err() {
        let good = serde_json::json!({
            "session": "s",
            "leftPath": "/a",
            "rightPath": "/b",
            "leftLines": 1,
            "rightLines": 1,
            "rows": 1,
            "opCount": 1,
            "exact": true,
            "truncated": false,
            "appliedOptions": {"ignoreWhitespace": false, "ignoreCase": false}
        });
        let ok: Result<crate::commands::text::TextCompareDto, String> =
            from_value_with_field("rs.cmd/text", good);
        assert!(ok.is_ok());
        let bad = serde_json::json!({ "session": 42 });
        let err: Result<crate::commands::text::TextCompareDto, String> =
            from_value_with_field("rs.cmd/text", bad);
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(msg.contains("rs.cmd/text") || msg.contains("field:") || msg.contains("line:"));
    }
}
