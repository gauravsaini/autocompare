//! Default-deny redaction: hashes and shapes only, never contents.

use sha2::{Digest, Sha256};

/// Shape-only summary of a byte buffer: truncated sha + size + line count.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ByteSummary {
    pub sha12: String,
    pub bytes: u64,
    pub lines: u64,
}

/// Summarize without ever retaining contents.
pub fn summarize_bytes(data: &[u8]) -> ByteSummary {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut sha12 = String::with_capacity(12);
    for b in digest.iter().take(6) {
        sha12.push_str(&format!("{b:02x}"));
    }
    let lines = data.iter().filter(|b| **b == b'\n').count() as u64;
    let bytes = data.len() as u64;
    ByteSummary { sha12, bytes, lines }
}

/// Hash an absolute path with the per-session salt. Returns a stable
/// opaque token; the raw path never leaves the process.
pub fn hash_path(path: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update([0u8]);
    hasher.update(path.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::from("p_");
    for b in digest.iter().take(8) {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

const SECRET_MARKERS: &[&str] = &["key", "token", "secret", "password", "bearer", "auth"];

/// True when free-form text looks like it carries a secret.
pub fn looks_like_secret(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    SECRET_MARKERS.iter().any(|m| lower.contains(m))
}

/// Truncate stderr-style output and refuse secret-looking content.
pub fn scrub_external_output(text: &str) -> Option<String> {
    if looks_like_secret(text) {
        return None;
    }
    let truncated: String = text.chars().take(500).collect();
    Some(truncated)
}
