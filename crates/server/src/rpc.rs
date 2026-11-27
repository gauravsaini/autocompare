//! JSON-RPC command dispatch for `autocompare-server`.

#![forbid(unsafe_code)]

use std::fs;

use autocompare_compare::format_converter::FormatConverterRegistry;
use autocompare_vfs::registry::VfsRegistry;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Maximum file/body read limit for RPC payloads (16 MiB).
pub const MAX_RPC_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Incoming JSON-RPC request payload for `POST /rpc`.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcRequest {
    /// Optional caller-supplied correlation identifier.
    #[serde(default)]
    pub id: Option<Value>,
    /// Command/method name matching Tauri IPC command conventions.
    pub method: String,
    /// JSON object of named parameters.
    #[serde(default)]
    pub params: Value,
}

/// Outgoing JSON-RPC response payload from `POST /rpc`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// Echoed correlation identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    /// `true` when the method completed without error.
    pub ok: bool,
    /// Successful JSON result payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Human-readable error message on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Dispatches an RPC command to the AutoCompare core crates.
///
/// # Errors
///
/// Returns an error message string if parameters are missing or the operation fails.
pub fn dispatch_rpc(method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "ping" | "health" => Ok(json!({
            "status": "ok",
            "transport": "http-rpc",
            "version": env!("CARGO_PKG_VERSION"),
        })),
        "vfs_schemes" => {
            let reg = VfsRegistry::with_builtins();
            Ok(json!({ "schemes": reg.schemes() }))
        }
        "vfs_list_dir" => {
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing string parameter 'uri'".to_string())?;
            let reg = VfsRegistry::with_builtins();
            let (vfs, subpath) = reg.open_with_subpath(uri).map_err(|e| e.to_string())?;
            let entries = vfs.list_dir(&subpath).map_err(|e| e.to_string())?;
            let mapped: Vec<Value> = entries
                .into_iter()
                .map(|e| {
                    json!({
                        "name": e.name,
                        "path": e.path.as_str(),
                        "is_dir": e.meta.is_dir,
                        "len": e.meta.len,
                    })
                })
                .collect();
            Ok(json!({ "entries": mapped }))
        }
        "read_text_file" => {
            let path = params
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing string parameter 'path'".to_string())?;
            let meta = fs::metadata(path).map_err(|e| format!("stat '{path}': {e}"))?;
            if meta.len() > MAX_RPC_BODY_BYTES as u64 {
                return Err(format!("file '{path}' exceeds 16 MiB read limit"));
            }
            let content = fs::read_to_string(path).map_err(|e| format!("read '{path}': {e}"))?;
            Ok(json!({ "path": path, "content": content }))
        }
        "format_convert" => {
            let path = params
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing string parameter 'path'".to_string())?;
            let content = params
                .get("content")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing string parameter 'content'".to_string())?;
            let reg = FormatConverterRegistry::with_builtins();
            match reg.convert(path, content.as_bytes()) {
                Ok(Some(bytes)) => Ok(json!({
                    "converted": true,
                    "content": String::from_utf8_lossy(&bytes),
                })),
                Ok(None) => Ok(json!({
                    "converted": false,
                    "content": content,
                })),
                Err(e) => Err(e.to_string()),
            }
        }
        other => Err(format!("unknown RPC method '{other}'")),
    }
}
