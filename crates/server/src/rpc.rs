//! JSON-RPC command dispatch for `autocompare-server`.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use autocompare_compare::format_converter::FormatConverterRegistry;
use autocompare_compare::{
    FolderOptions, SyncDirection, SyncOptions, TableOptions, TextOptions, compare_folders,
    compare_table_files, compare_text_files, plan_sync,
};
use autocompare_diff::Op;
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

fn req_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string parameter '{key}'"))
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
            let uri = req_str(params, "uri")?;
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
            let path = req_str(params, "path")?;
            let meta = fs::metadata(path).map_err(|e| format!("stat '{path}': {e}"))?;
            if meta.len() > MAX_RPC_BODY_BYTES as u64 {
                return Err(format!("file '{path}' exceeds 16 MiB read limit"));
            }
            let content = fs::read_to_string(path).map_err(|e| format!("read '{path}': {e}"))?;
            Ok(json!({ "path": path, "content": content }))
        }
        "format_convert" => {
            let path = req_str(params, "path")?;
            let content = req_str(params, "content")?;
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
        "compare_text" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let opts = TextOptions {
                ignore_case: params
                    .get("ignore_case")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                ignore_space_change: params
                    .get("ignore_space_change")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                ..Default::default()
            };
            let cmp = compare_text_files(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            let equal = cmp
                .script
                .ops
                .iter()
                .all(|op| matches!(op, Op::Equal { .. }));
            Ok(json!({
                "left_lines": cmp.left_lines,
                "right_lines": cmp.right_lines,
                "left_count": cmp.left_lines.len(),
                "right_count": cmp.right_lines.len(),
                "equal": equal,
            }))
        }
        "compare_folders" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let opts = FolderOptions {
                content_compare: params
                    .get("content_compare")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                case_insensitive: params
                    .get("case_insensitive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                depth_limit: params
                    .get("depth_limit")
                    .and_then(Value::as_u64)
                    .and_then(|d| usize::try_from(d).ok())
                    .unwrap_or(32),
                ..Default::default()
            };
            let cmp = compare_folders(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&cmp).map_err(|e| e.to_string())
        }
        "compare_table" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let delimiter = params
                .get("delimiter")
                .and_then(Value::as_str)
                .and_then(|s| s.chars().next())
                .unwrap_or(',');
            let header = params
                .get("header")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let opts = TableOptions { delimiter, header };
            let cmp = compare_table_files(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "header_l": cmp.header_l,
                "header_r": cmp.header_r,
                "rows_l": cmp.rows_l,
                "rows_r": cmp.rows_r,
                "equal": cmp.script.ops.iter().all(|op| matches!(op, Op::Equal { .. })),
            }))
        }
        "plan_sync" => {
            let left = req_str(params, "left")?;
            let right = req_str(params, "right")?;
            let direction = match params
                .get("direction")
                .and_then(Value::as_str)
                .unwrap_or("l2r")
            {
                "r2l" | "right_to_left" => SyncDirection::RightToLeft,
                "newer" | "newer_wins" => SyncDirection::NewerWins,
                _ => SyncDirection::LeftToRight,
            };
            let prune = params
                .get("prune")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let opts = SyncOptions { direction, prune };
            let plan = plan_sync(Path::new(left), Path::new(right), &opts)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&plan).map_err(|e| e.to_string())
        }
        other => Err(format!("unknown RPC method '{other}'")),
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
    fn dispatches_compare_text_table_folder_and_plan_sync() {
        let base = std::env::temp_dir().join(format!(
            "autocompare_rpc_test_{}",
            std::process::id()
        ));
        let left_dir = base.join("left");
        let right_dir = base.join("right");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&left_dir).expect("mkdir left");
        fs::create_dir_all(&right_dir).expect("mkdir right");

        let l_txt = left_dir.join("a.txt");
        let r_txt = right_dir.join("a.txt");
        fs::write(&l_txt, "hello\nworld\n").expect("write l_txt");
        fs::write(&r_txt, "hello\nrust\n").expect("write r_txt");

        let l_csv = left_dir.join("t.csv");
        let r_csv = right_dir.join("t.csv");
        fs::write(&l_csv, "id,name\n1,Alice\n").expect("write l_csv");
        fs::write(&r_csv, "id,name\n1,Alice\n").expect("write r_csv");

        let txt_res = dispatch_rpc(
            "compare_text",
            &json!({ "left": l_txt.to_string_lossy(), "right": r_txt.to_string_lossy() }),
        )
        .expect("compare_text");
        assert_eq!(txt_res.get("equal"), Some(&json!(false)));
        assert_eq!(txt_res.get("left_count"), Some(&json!(2)));

        let tbl_res = dispatch_rpc(
            "compare_table",
            &json!({ "left": l_csv.to_string_lossy(), "right": r_csv.to_string_lossy() }),
        )
        .expect("compare_table");
        assert_eq!(tbl_res.get("equal"), Some(&json!(true)));

        let fld_res = dispatch_rpc(
            "compare_folders",
            &json!({ "left": left_dir.to_string_lossy(), "right": right_dir.to_string_lossy() }),
        )
        .expect("compare_folders");
        assert!(fld_res.get("entries").and_then(Value::as_array).is_some());

        let sync_res = dispatch_rpc(
            "plan_sync",
            &json!({ "left": left_dir.to_string_lossy(), "right": right_dir.to_string_lossy(), "direction": "l2r" }),
        )
        .expect("plan_sync");
        assert!(sync_res.get("steps").and_then(Value::as_array).is_some());

        let _ = fs::remove_dir_all(&base);
    }
}
