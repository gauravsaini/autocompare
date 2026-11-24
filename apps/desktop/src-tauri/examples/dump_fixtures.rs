//! Wave-4A real-data fixture dumper (example binary, NOT shipped).
//!
//! Calls the real engine (`text_compare`, `folder_compare`, `table_compare`
//! shapes via `autocompare-compare` + the desktop `build_rows` row builder)
//! over small built-in input pairs and prints ONE JSON document to stdout:
//! `{ "textRows": [...], "folderNodes": [...], "tableCells": {...} }`.
//! Diagnostics go to stderr. The committed `dumped.json` is this output.
//!
//! Thresholds (asserted, fail loudly instead of dumping small fixtures):
//! text >= 600 rows (incl. equal/changed/leftOnly/rightOnly runs),
//! folder >= 550 nodes, table >= 1500 cells.

use std::path::{Path, PathBuf};

use autocompare_compare::{
    FolderOptions, FolderStatus, TableOptions, TextOptions, compare_folders,
    compare_table_files, compare_text_files,
};
use autocompare_desktop_lib::commands::folder::{FolderEntryDto, FolderStatusDto, NodeInfoDto};
use autocompare_desktop_lib::commands::text::build_rows;

const MIN_TEXT_ROWS: usize = 600;
const MIN_FOLDER_NODES: usize = 550;
const MIN_TABLE_CELLS: usize = 1500;

fn fail(msg: &str) -> ! {
    eprintln!("dump_fixtures: {msg}");
    std::process::exit(1);
}

fn scratch_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("wave4a-fixtures-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap_or(());
    }
    std::fs::create_dir_all(&root).unwrap_or_else(|e| fail(&format!("mkdir scratch: {e}")));
    root
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| fail(&format!("mkdir: {e}")));
    }
    std::fs::write(path, content).unwrap_or_else(|e| fail(&format!("write {}: {e}", path.display())));
}

// ---- text: 640 left lines, right drops 4 (leftOnly run), inserts 4
// ---- (rightOnly run), substitutes every 47th line (changed singles).
fn dump_text(root: &Path) -> Vec<serde_json::Value> {
    let mut left: Vec<String> = Vec::new();
    for i in 0..640 {
        left.push(format!("common line {i:04} with stable body text"));
    }
    // Left-only run: lines 6..10 vanish on the right.
    for (k, slot) in left.iter_mut().skip(6).take(4).enumerate() {
        *slot = format!("removed line {}", (b'A' + k as u8) as char);
    }
    // Changed singles every 47 lines (kept clear of the runs above).
    for i in (47..640).step_by(47) {
        if !(6..25).contains(&i)
            && let Some(slot) = left.get_mut(i)
        {
            *slot = format!("left variant of line {i:04}");
        }
    }

    let mut right: Vec<String> = Vec::new();
    for (i, line) in left.iter().enumerate() {
        if (6..10).contains(&i) {
            continue; // leftOnly run
        }
        if (47..640).step_by(47).any(|c| c == i) && !(6..25).contains(&i) {
            right.push(format!("right variant of line {i:04}"));
        } else {
            right.push(line.clone());
        }
        if i == 19 {
            // Right-only run, anchored by equal lines on both sides.
            for roman in ["I", "II", "III", "IV"] {
                right.push(format!("inserted extra line {roman}"));
            }
        }
    }

    let lpath = root.join("text_left.txt");
    let rpath = root.join("text_right.txt");
    write_file(&lpath, &left.join("\n"));
    write_file(&rpath, &right.join("\n"));

    let compared = compare_text_files(&lpath, &rpath, &TextOptions::default())
        .unwrap_or_else(|e| fail(&format!("text compare: {e}")));
    let rows = build_rows(&compared.left_lines, &compared.right_lines, &compared.script);
    if rows.len() < MIN_TEXT_ROWS {
        fail(&format!("text rows {} < {MIN_TEXT_ROWS}", rows.len()));
    }
    let kinds: std::collections::HashSet<String> =
        rows.iter().map(|r| format!("{:?}", r.kind)).collect();
    for need in ["Equal", "Changed", "LeftOnly", "RightOnly"] {
        if !kinds.contains(need) {
            fail(&format!("text rows miss kind {need}: {kinds:?}"));
        }
    }
    eprintln!(
        "dump_fixtures: text rows={} kinds={:?}",
        rows.len(),
        kinds
    );
    rows.iter()
        .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
        .collect()
}

// ---- folder: 20 dirs x 27 files + 2 top-level files on the left;
// ---- right drops d05, adds d20, resizes 10 files (different).
fn dump_folder(root: &Path) -> Vec<serde_json::Value> {
    let lroot = root.join("folder_left");
    let rroot = root.join("folder_right");
    for d in 0..20 {
        for f in 0..27 {
            let body = format!("content of d{d:02}/f{f:02} {}", "x".repeat(40 + ((d * 27 + f) % 90)));
            write_file(&lroot.join(format!("d{d:02}/f{f:02}.txt")), &body);
        }
    }
    write_file(&lroot.join("readme.txt"), "top-level readme\n");
    write_file(&lroot.join("notes.txt"), "top-level notes\n");

    // Mirror, then mutate the right side. Names are chosen so the one-sided
    // runs sort EARLY (d00 dropped, d00a added): the first 50-entry page
    // already carries leftOnly + rightOnly verdicts for the DOM to show.
    copy_dir(&lroot, &rroot);
    std::fs::remove_dir_all(rroot.join("d00")).unwrap_or_else(|e| fail(&format!("rm d00: {e}")));
    for f in 0..27 {
        write_file(
            &rroot.join(format!("d00a/f{f:02}.txt")),
            &format!("brand new right-only file {f:02} {}", "y".repeat(60 + f)),
        );
    }
    for d in 2..12 {
        write_file(
            &rroot.join(format!("d{d:02}/f{d:02}.txt")),
            &format!("resized replacement content {}", "z".repeat(200 + d * 7)),
        );
    }

    let compared = compare_folders(&lroot, &rroot, &FolderOptions::default())
        .unwrap_or_else(|e| fail(&format!("folder compare: {e}")));
    if compared.entries.len() < MIN_FOLDER_NODES {
        fail(&format!(
            "folder nodes {} < {MIN_FOLDER_NODES}",
            compared.entries.len()
        ));
    }
    let dtos: Vec<FolderEntryDto> = compared.entries.iter().map(to_dto).collect();
    let statuses: std::collections::HashSet<String> =
        dtos.iter().map(|e| format!("{:?}", e.status)).collect();
    for need in ["LeftOnly", "RightOnly", "Different"] {
        if !statuses.contains(need) {
            fail(&format!("folder nodes miss status {need}: {statuses:?}"));
        }
    }
    eprintln!(
        "dump_fixtures: folder nodes={} statuses={:?}",
        dtos.len(),
        statuses
    );
    dtos.iter()
        .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
        .collect()
}

fn copy_dir(src: &Path, dst: &Path) {
    let mut stack = vec![src.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rel = dir.strip_prefix(src).unwrap_or(dir.as_path());
        std::fs::create_dir_all(dst.join(rel)).unwrap_or_else(|e| fail(&format!("mkdir: {e}")));
        let mut items: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| fail(&format!("read_dir: {e}")))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        items.sort();
        for item in items {
            if item.is_dir() {
                stack.push(item);
            } else {
                let rel = item.strip_prefix(src).unwrap_or(item.as_path());
                std::fs::copy(&item, dst.join(rel))
                    .unwrap_or_else(|e| fail(&format!("copy: {e}")));
            }
        }
    }
}

fn to_dto(entry: &autocompare_compare::FolderEntry) -> FolderEntryDto {
    let status = match entry.status {
        FolderStatus::Same => FolderStatusDto::Same,
        FolderStatus::Different => FolderStatusDto::Different,
        FolderStatus::NewerLeft => FolderStatusDto::NewerLeft,
        FolderStatus::NewerRight => FolderStatusDto::NewerRight,
        FolderStatus::LeftOnly => FolderStatusDto::LeftOnly,
        FolderStatus::RightOnly => FolderStatusDto::RightOnly,
    };
    let info = |n: &Option<autocompare_compare::NodeInfo>| {
        n.as_ref().map(|v| NodeInfoDto {
            size: v.size,
            modified_ms: v
                .modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_millis() as u64),
        })
    };
    FolderEntryDto {
        rel_path: entry.rel_path.clone(),
        is_dir: entry.is_dir,
        left: info(&entry.left),
        right: info(&entry.right),
        status,
    }
}

// ---- table: 550 x 3 cells per side; right edits ~25 cells,
// ---- drops 5 rows, appends 5 rows.
fn dump_table(root: &Path) -> serde_json::Value {
    let header = "id,name,price";
    let mut left_rows: Vec<String> = Vec::new();
    for i in 1..=550 {
        left_rows.push(format!("R{i:04},item-{i:04},{:.2}", 10.0 + i as f64 * 0.25));
    }
    let mut right_rows = left_rows.clone();
    for (n, i) in (5..550).step_by(23).take(25).enumerate() {
        let new_row = right_rows.get(i).and_then(|row| {
            let mut parts = row.split(',');
            let id = parts.next()?;
            let name = parts.next()?;
            Some(format!("{id},{name},{:.2}", 99.0 + n as f64))
        });
        if let (Some(new_row), Some(row)) = (new_row, right_rows.get_mut(i)) {
            *row = new_row;
        }
    }
    right_rows.drain(100..105); // leftOnly rows
    for k in 0..5 {
        // rightOnly rows
        right_rows.insert(
            200 + k,
            format!("X{k:04},new-item-{k:04},{:.2}", 500.0 + k as f64),
        );
    }

    let lpath = root.join("table_left.csv");
    let rpath = root.join("table_right.csv");
    write_file(&lpath, &(header.to_string() + "\n" + &left_rows.join("\n")));
    write_file(&rpath, &(header.to_string() + "\n" + &right_rows.join("\n")));

    let compared = compare_table_files(&lpath, &rpath, &TableOptions::default())
        .unwrap_or_else(|e| fail(&format!("table compare: {e}")));
    let cells = compared.rows_l.len() * 3 + compared.rows_r.len() * 3;
    if cells < MIN_TABLE_CELLS {
        fail(&format!("table cells {cells} < {MIN_TABLE_CELLS}"));
    }
    let total_cols = compared
        .rows_l
        .iter()
        .chain(compared.rows_r.iter())
        .map(|r| r.len())
        .max()
        .unwrap_or(0)
        .max(compared.header_l.len())
        .max(compared.header_r.len());
    eprintln!(
        "dump_fixtures: table rowsL={} rowsR={} cols={} cells={}",
        compared.rows_l.len(),
        compared.rows_r.len(),
        total_cols,
        cells
    );
    serde_json::json!({
        "header": compared.header_l,
        "headerR": compared.header_r,
        "rowsL": compared.rows_l,
        "rowsR": compared.rows_r,
        "totalRowsL": compared.rows_l.len(),
        "totalRowsR": compared.rows_r.len(),
        "totalCols": total_cols,
        "leftCols": compared.rows_l.iter().map(|r| r.len()).max().unwrap_or(0),
        "rightCols": compared.rows_r.iter().map(|r| r.len()).max().unwrap_or(0),
        "equal": compared.rows_l == compared.rows_r && compared.header_l == compared.header_r,
    })
}

fn main() {
    let root = scratch_root();
    let text_rows = dump_text(&root);
    let folder_nodes = dump_folder(&root);
    let table_cells = dump_table(&root);
    let doc = serde_json::json!({
        "textRows": text_rows,
        "folderNodes": folder_nodes,
        "tableCells": table_cells,
    });
    println!("{}", serde_json::to_string(&doc).unwrap_or_default());
    std::fs::remove_dir_all(&root).unwrap_or(());
}
