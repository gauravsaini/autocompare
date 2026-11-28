//! ODD Phase 6 orchestration helpers for `autocompare-diff`.
//!
//! One span per public diff entry function (3-depth rule: command ->
//! orchestration -> algorithm). Leaf helpers in `engine`, `word_diff`
//! internals, and per-item loops stay uninstrumented.
//!
//! Every attribute is a redacted summary: line/byte counts, option
//! flags, op counts. File contents, line text, and raw paths never
//! enter telemetry.

#![forbid(unsafe_code)]

/// True when telemetry emission is enabled (`AUTOCOMPARE_TELEMETRY != off`).
pub(crate) fn enabled() -> bool {
    autocompare_telemetry::Sampling::enabled()
}

/// True when merged output contains conflict markers.
pub(crate) fn has_conflict_markers(lines: &[String]) -> bool {
    lines.iter().any(|l| {
        l.starts_with("<<<<<<<") || l.starts_with("=======") || l.starts_with(">>>>>>>")
    })
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
    use std::sync::{Arc, Mutex};
    use tracing::subscriber::with_default;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::registry::LookupSpan;

    #[derive(Debug, Clone, Default)]
    struct CapturedSpan {
        name: String,
        fields: Vec<String>,
    }

    #[derive(Debug, Clone, Default)]
    struct Capture {
        spans: Arc<Mutex<Vec<CapturedSpan>>>,
    }

    struct CaptureLayer {
        capture: Capture,
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            id: &tracing::span::Id,
            ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut values: Vec<String> = Vec::new();
            struct Visitor<'a> {
                values: &'a mut Vec<String>,
            }
            impl<'a> tracing::field::Visit for Visitor<'a> {
                fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                    let rendered = format!("{field}={value:?}");
                    self.values.push(rendered);
                }
                fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                    self.values.push(format!("{field}={value}"));
                }
                fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                    self.values.push(format!("{field}={value}"));
                }
                fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                    self.values.push(format!("{field}={value}"));
                }
            }
            let mut visitor = Visitor { values: &mut values };
            attrs.record(&mut visitor);
            if let Some(span) = ctx.span(id) {
                let mut ext = span.extensions_mut();
                ext.insert(values.clone());
            }
            if let Ok(mut guard) = self.capture.spans.lock() {
                guard.push(CapturedSpan {
                    name: attrs.metadata().name().to_string(),
                    fields: values,
                });
            }
        }
        fn on_record(
            &self,
            id: &tracing::span::Id,
            values: &tracing::span::Record<'_>,
            ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct VecVisitor<'a> {
                values: &'a mut Vec<String>,
            }
            impl<'a> tracing::field::Visit for VecVisitor<'a> {
                fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                    self.values.push(format!("{field}={value:?}"));
                }
                fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                    self.values.push(format!("{field}={value}"));
                }
                fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                    self.values.push(format!("{field}={value}"));
                }
            }
            if let Some(span) = ctx.span(id) {
                let mut ext = span.extensions_mut();
                if let Some(stored) = ext.get_mut::<Vec<String>>() {
                    let mut visitor = VecVisitor { values: stored };
                    values.record(&mut visitor);
                }
            }
            // Also mirror into the flat capture list for presence asserts.
            if let Ok(mut guard) = self.capture.spans.lock() {
                let mut extra: Vec<String> = Vec::new();
                let mut visitor = VecVisitor { values: &mut extra };
                values.record(&mut visitor);
                guard.push(CapturedSpan {
                    name: "__record__".to_string(),
                    fields: extra,
                });
            }
        }
    }

    fn with_capture<T>(f: impl FnOnce(Capture) -> T) -> T {
        let capture = Capture::default();
        let layer = CaptureLayer {
            capture: capture.clone(),
        };
        let sub = tracing_subscriber::registry().with(layer);
        with_default(sub, || f(capture))
    }

    fn span_names(capture: &Capture) -> Vec<String> {
        capture
            .spans
            .lock()
            .map(|v| v.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default()
    }

    fn all_fields(capture: &Capture) -> String {
        capture
            .spans
            .lock()
            .map(|v| {
                v.iter()
                    .flat_map(|s| s.fields.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    #[test]
    fn span_presence_per_diff_op() {
        with_capture(|capture| {
            let a = ["alpha", "beta", "gamma"];
            let b = ["alpha", "BETA", "gamma"];
            let _ = crate::diff_lines(&a, &b);
            let _ = crate::inline_diff("hello world", "hello rust");
            let _ = crate::merge3(&a, &a, &b);
            let _ = crate::merge3_with_algorithm(&a, &a, &b, crate::text_diff::DiffAlgorithm::Myers);
            let _ = crate::text_diff::diff(&a, &b, crate::text_diff::DiffAlgorithm::Myers);
            let _ = crate::hex::diff_hex(b"abc", b"abd", crate::hex::HexAlignment::None);
            let _ = crate::hex::diff_hex_fast(b"abc", b"abd");
            let _ = crate::hex::diff_hex_complete(b"abc", b"abd");
            let _ = crate::table::diff_rows(
                &[vec!["a".to_string()]],
                &[vec!["b".to_string()]],
            );
            let left_img = crate::RgbaImage {
                width: 2,
                height: 1,
                pixels: vec![255, 0, 0, 255, 0, 255, 0, 255],
            };
            let right_img = crate::RgbaImage {
                width: 2,
                height: 1,
                pixels: vec![255, 0, 0, 255, 0, 0, 255, 255],
            };
            let _ = crate::diff_pictures(&left_img, &right_img);
            let names = span_names(&capture);
            for expected in [
                "diff.lines",
                "diff.inline",
                "diff.merge3",
                "diff.text",
                "diff.hex",
                "diff.hex_fast",
                "diff.hex_complete",
                "diff.table_rows",
                "diff.pictures",
            ] {
                assert!(
                    names.iter().any(|n| n == expected),
                    "expected span {expected}, got {names:?}"
                );
            }
        });
    }

    #[test]
    fn redaction_never_emits_contents() {
        let secret = "super-secret-token password=hunter2 bearer abc123";
        with_capture(|capture| {
            let a = ["public line one", secret];
            let b = ["public line one", "public line two"];
            let _ = crate::diff_lines(&a, &b);
            let _ = crate::inline_diff(secret, "public line two");
            let _ = crate::text_diff::diff(&a, &b, crate::text_diff::DiffAlgorithm::Myers);
            let _ = crate::hex::diff_hex(secret.as_bytes(), b"public", crate::hex::HexAlignment::None);
            let blob = all_fields(&capture);
            assert!(
                !blob.contains("hunter2"),
                "span fields leaked secret contents: {blob}"
            );
            assert!(
                !blob.contains("super-secret-token"),
                "span fields leaked secret contents: {blob}"
            );
            assert!(
                !blob.contains("public line one"),
                "span fields leaked line contents: {blob}"
            );
        });
    }

    #[test]
    fn counts_are_honest() {
        assert!(!has_conflict_markers(&["clean".to_string()]));
        assert!(has_conflict_markers(&["<<<<<<< left".to_string()]));
    }
}
