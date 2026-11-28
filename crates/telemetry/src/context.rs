//! W3C-compatible trace context propagation.

use serde::{Deserialize, Serialize};

/// 128-bit trace id + 64-bit span id + sampling flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub sampled: bool,
}

impl TraceContext {
    /// Fresh random context. Falls back to a time-seeded id when the OS
    /// RNG is unavailable so context creation never fails.
    pub fn fresh(sampled: bool) -> Self {
        let mut trace_id = [0u8; 16];
        let mut span_id = [0u8; 8];
        if getrandom::fill(&mut trace_id).is_err() {
            mix_time(&mut trace_id);
        }
        if getrandom::fill(&mut span_id).is_err() {
            mix_time(&mut span_id);
        }
        Self { trace_id, span_id, sampled }
    }

    /// Child span under the same trace.
    pub fn child(&self) -> Self {
        let mut child = Self::fresh(self.sampled);
        child.trace_id = self.trace_id;
        child
    }

    /// Parse a W3C `traceparent` header (`00-<trace>-<span>-<flags>`).
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let mut parts = header.split('-');
        if parts.next()? != "00" {
            return None;
        }
        let trace_hex = parts.next()?;
        let span_hex = parts.next()?;
        let flags_hex = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            trace_id: decode_hex_16(trace_hex)?,
            span_id: decode_hex_8(span_hex)?,
            sampled: flags_hex.strip_prefix('0').is_some_and(|f| f == "1")
                || flags_hex == "01",
        })
    }

    /// Render a W3C `traceparent` header.
    pub fn to_traceparent(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", hex_of(&self.trace_id), hex_of(&self.span_id), flags)
    }

    /// Inject into a Tauri invoke params envelope (`__trace` field).
    pub fn to_envelope(&self) -> serde_json::Value {
        serde_json::json!({
            "trace_id": hex_of(&self.trace_id),
            "span_id": hex_of(&self.span_id),
            "sampled": self.sampled,
        })
    }

    /// Extract from a Tauri invoke params envelope. Returns `None` when
    /// absent or malformed — callers must start a fresh root in that case.
    pub fn from_envelope(value: &serde_json::Value) -> Option<Self> {
        let obj = value.get("__trace")?;
        let trace_hex = obj.get("trace_id")?.as_str()?;
        let span_hex = obj.get("span_id")?.as_str()?;
        Some(Self {
            trace_id: decode_hex_16(trace_hex)?,
            span_id: decode_hex_8(span_hex)?,
            sampled: obj.get("sampled").and_then(serde_json::Value::as_bool).unwrap_or(false),
        })
    }
}

fn hex_of(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let hi = HEX.get((b >> 4) as usize).copied().unwrap_or(b'?');
        let lo = HEX.get((b & 0x0f) as usize).copied().unwrap_or(b'?');
        out.push(hi as char);
        out.push(lo as char);
    }
    out
}

fn decode_hex_16(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = hex_val(chunk.first()?)?;
        let lo = hex_val(chunk.get(1)?)?;
        let slot = out.get_mut(i)?;
        *slot = (hi << 4) | lo;
    }
    if out.iter().all(|b| *b == 0) {
        return None;
    }
    Some(out)
}

fn decode_hex_8(s: &str) -> Option<[u8; 8]> {
    if s.len() != 16 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 8];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = hex_val(chunk.first()?)?;
        let lo = hex_val(chunk.get(1)?)?;
        let slot = out.get_mut(i)?;
        *slot = (hi << 4) | lo;
    }
    if out.iter().all(|b| *b == 0) {
        return None;
    }
    Some(out)
}

fn hex_val(b: &u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn mix_time(buf: &mut [u8]) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0x9e3779b97f4a7c15);
    let pid = std::process::id() as u128;
    let mut x = nanos ^ (pid << 64) ^ 0x9e3779b97f4a7c159e3779b97f4a7c15u128;
    for chunk in buf.chunks_mut(8) {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x = x.wrapping_mul(0x2545F4914F6CDD1D);
        for (i, slot) in chunk.iter_mut().enumerate() {
            if let Some(shift) = u32::try_from(i * 8).ok().and_then(|s| 64u32.checked_sub(s + 8)) {
                *slot ^= ((x >> shift) & 0xff) as u8;
            }
        }
    }
}
