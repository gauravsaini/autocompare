//! ODD Phase 6 I/O summary helpers for `autocompare-vfs`.
//!
//! Spans record file/byte counts and archive entry counts only.
//! File names, directory names, and file contents never enter
//! telemetry. Path identity is an opaque salted hash (allowlisted),
//! never the raw path.

#![forbid(unsafe_code)]

/// True when telemetry emission is enabled (`AUTOCOMPARE_TELEMETRY != off`).
pub(crate) fn enabled() -> bool {
    autocompare_telemetry::Sampling::enabled()
}

/// Process-unique salt: stable in-process so one run correlates,
/// never persisted anywhere.
pub(crate) fn salt() -> String {
    format!("odd-vfs-v1-{}", std::process::id())
}

/// Opaque token for a filesystem path. The raw path never leaves the process.
pub(crate) fn path_token(path: &str) -> String {
    autocompare_telemetry::redact::hash_path(path, &salt())
}

/// Opaque token for a URI scheme + authority shape (e.g. `zip`, `file`).
/// Schemes are allowlisted tokens, not names/contents.
pub(crate) fn scheme_token(scheme: &str) -> String {
    let norm = scheme.trim().to_ascii_lowercase();
    if norm.is_empty() {
        return "unknown".to_string();
    }
    let mut out = String::with_capacity(norm.len());
    for ch in norm.chars() {
        if ch.is_ascii_alphanumeric() || ch == '+' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 24 {
            break;
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::trait_def::Vfs;
    use std::sync::{Arc, Mutex};
    use tracing::subscriber::with_default;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::registry::LookupSpan;

    #[derive(Debug, Clone, Default)]
    struct Captured {
        names: Arc<Mutex<Vec<String>>>,
        fields: Arc<Mutex<Vec<String>>>,
    }

    struct CaptureLayer {
        capture: Captured,
    }

    impl<S> tracing_subscriber::Layer<S> for CaptureLayer
    where
        S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            _id: &tracing::span::Id,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct V<'a> {
                out: &'a Mutex<Vec<String>>,
            }
            impl<'a> tracing::field::Visit for V<'a> {
                fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v:?}"));
                    }
                }
                fn record_str(&mut self, f: &tracing::field::Field, v: &str) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v}"));
                    }
                }
                fn record_u64(&mut self, f: &tracing::field::Field, v: u64) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v}"));
                    }
                }
                fn record_bool(&mut self, f: &tracing::field::Field, v: bool) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v}"));
                    }
                }
            }
            if let Ok(mut g) = self.capture.names.lock() {
                g.push(attrs.metadata().name().to_string());
            }
            let mut v = V {
                out: &self.capture.fields,
            };
            attrs.record(&mut v);
        }
        fn on_record(
            &self,
            _id: &tracing::span::Id,
            values: &tracing::span::Record<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct V<'a> {
                out: &'a Mutex<Vec<String>>,
            }
            impl<'a> tracing::field::Visit for V<'a> {
                fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v:?}"));
                    }
                }
                fn record_u64(&mut self, f: &tracing::field::Field, v: u64) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v}"));
                    }
                }
                fn record_bool(&mut self, f: &tracing::field::Field, v: bool) {
                    if let Ok(mut g) = self.out.lock() {
                        g.push(format!("{f}={v}"));
                    }
                }
            }
            let mut v = V {
                out: &self.capture.fields,
            };
            values.record(&mut v);
        }
    }

    fn with_capture(f: impl FnOnce(Captured)) {
        let capture = Captured::default();
        let layer = CaptureLayer {
            capture: capture.clone(),
        };
        let sub = tracing_subscriber::registry().with(layer);
        with_default(sub, || f(capture));
    }

    #[test]
    fn span_presence_per_vfs_op() {
        with_capture(|capture| {
            let dir = std::env::temp_dir().join(format!("odd-vfs-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join("hello.txt");
            let _ = std::fs::write(&file, b"hello vfs");
            let scheme = crate::registry::detect_scheme_from_path("archive.zip");
            let _ = scheme_token(scheme);
            let fs = crate::LocalFs::new(&dir).expect("localfs");
            let root = crate::VfsPath::root();
            let _ = fs.list_dir(&root);
            let _ = fs.stat(&root);
            let rel = crate::VfsPath::new("hello.txt").expect("vfs path");
            let _ = fs.read_stream(&rel, 1024 * 1024);
            let names = capture.names.lock().map(|v| v.clone()).unwrap_or_default();
            for expected in ["vfs.list_dir", "vfs.stat", "vfs.read"] {
                assert!(
                    names.iter().any(|n| n == expected),
                    "expected span {expected}, got {names:?}"
                );
            }
            let _ = std::fs::remove_dir_all(&dir);
        });
    }

    #[test]
    fn redaction_never_emits_names_or_contents() {
        let secret_name = "super-secret-token-password-hunter2.txt";
        let secret_bytes = b"super-secret-contents bearer abc123";
        with_capture(|capture| {
            let dir = std::env::temp_dir().join(format!("odd-vfs-redact-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join(secret_name);
            let _ = std::fs::write(&file, secret_bytes);
            let fs = crate::LocalFs::new(&dir).expect("localfs");
            let rel = crate::VfsPath::new(secret_name).expect("vfs path");
            let _ = fs.read_stream(&rel, 1024 * 1024);
            let _ = fs.list_dir(&crate::VfsPath::root());
            let fields = capture
                .fields
                .lock()
                .map(|v| v.join("\n"))
                .unwrap_or_default();
            assert!(
                !fields.contains("hunter2"),
                "vfs span leaked secret name/contents: {fields}"
            );
            assert!(
                !fields.contains("super-secret-contents"),
                "vfs span leaked contents: {fields}"
            );
            assert!(
                !fields.contains(secret_name),
                "vfs span leaked raw file name: {fields}"
            );
            let _ = std::fs::remove_dir_all(&dir);
        });
        // Path tokens are opaque and stable per process.
        let t1 = path_token("/some/secret/path.txt");
        let t2 = path_token("/some/secret/path.txt");
        assert_eq!(t1, t2);
        assert!(t1.starts_with("p_"));
        assert!(!t1.contains("secret"));
    }
}
