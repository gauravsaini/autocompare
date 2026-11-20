//! Pure-Rust Zip archive virtual filesystem reader and writer.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::error::VfsError;
use crate::path::VfsPath;
use crate::trait_def::{Vfs, VfsEntry, VfsMeta, WalkOptions};

#[derive(Debug, Clone)]
struct ZipItemInfo {
    meta: VfsMeta,
    zip_index: usize,
}

/// A read-only virtual filesystem reader for standard `.zip` archives.
#[derive(Debug, Clone)]
pub struct ZipFs {
    data: Arc<Vec<u8>>,
    comment: Option<String>,
    entries: BTreeMap<VfsPath, ZipItemInfo>,
    dir_children: BTreeMap<VfsPath, Vec<VfsEntry>>,
}

/// Creates a new in-memory zip archive from a slice of relative paths and byte payloads.
///
/// # Errors
///
/// Returns [`VfsError::CorruptedArchive`] if zip construction fails.
pub fn create_zip_archive(entries: &[(&VfsPath, &[u8])]) -> Result<Vec<u8>, VfsError> {
    let mut buffer = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
    let options = SimpleFileOptions::default();

    for (path, data) in entries {
        if path.is_root() {
            continue;
        }
        writer
            .start_file(path.as_str(), options)
            .map_err(VfsError::from)?;
        writer.write_all(data).map_err(VfsError::Io)?;
    }
    writer.finish().map_err(VfsError::from)?;
    Ok(buffer)
}

/// Archives a virtual filesystem tree rooted at `root` into a standard Zip byte vector.
///
/// # Errors
///
/// Returns VFS or I/O errors encountered during directory traversal and archiving.
pub fn create_zip_from_vfs(
    vfs: &dyn Vfs,
    root: &VfsPath,
    options: &WalkOptions,
) -> Result<Vec<u8>, VfsError> {
    let entries = vfs.walk(root, options)?;
    let mut buffer = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
    let file_options = SimpleFileOptions::default();

    for entry in entries {
        let rel_path = if root.is_root() {
            entry.path.as_str().to_string()
        } else {
            match entry.path.strip_prefix(root) {
                Some(stripped) => {
                    if stripped.is_root() {
                        continue;
                    }
                    stripped.as_str().to_string()
                }
                None => {
                    if entry.path == *root {
                        continue;
                    }
                    entry.path.as_str().to_string()
                }
            }
        };

        if rel_path.is_empty() {
            continue;
        }

        if entry.meta.is_dir {
            let dir_name = if rel_path.ends_with('/') {
                rel_path
            } else {
                format!("{rel_path}/")
            };
            writer
                .add_directory(&dir_name, file_options)
                .map_err(VfsError::from)?;
        } else {
            writer
                .start_file(&rel_path, file_options)
                .map_err(VfsError::from)?;
            let content = vfs.read_all(&entry.path, 1024 * 1024 * 64)?;
            writer.write_all(&content).map_err(VfsError::Io)?;
        }
    }
    writer.finish().map_err(VfsError::from)?;
    Ok(buffer)
}

impl ZipFs {
    /// Opens a zip archive from a local file path.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if missing, or [`VfsError::CorruptedArchive`]
    /// if archive parsing fails.
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self, VfsError> {
        let bytes = fs::read(path.as_ref()).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(VfsPath::root()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(VfsPath::root()),
            _ => VfsError::Io(err),
        })?;
        Self::from_bytes(bytes)
    }

    /// Creates a [`ZipFs`] from an in-memory byte buffer.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::CorruptedArchive`] if the buffer is not a valid zip archive.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, VfsError> {
        let cursor = Cursor::new(&bytes);
        let mut archive = ZipArchive::new(cursor).map_err(VfsError::from)?;
        let comment = std::str::from_utf8(archive.comment())
            .ok()
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);

        let mut entries = BTreeMap::new();
        let mut dir_children: BTreeMap<VfsPath, Vec<VfsEntry>> = BTreeMap::new();
        let mut known_dirs = BTreeSet::new();
        known_dirs.insert(VfsPath::root());

        let count = archive.len();
        for index in 0..count {
            let file = archive.by_index(index).map_err(VfsError::from)?;
            let raw_name = file.name();

            // Skip internal zip artifacts like __MACOSX
            if raw_name.starts_with("__MACOSX/") || raw_name.contains("/__MACOSX/") {
                continue;
            }

            let path = match VfsPath::new(raw_name) {
                Ok(p) => p,
                Err(_) => continue, // Ignore invalid path names (e.g. traversal)
            };

            if path.is_root() {
                continue;
            }

            let unix_mode = file.unix_mode();
            let is_symlink = unix_mode.is_some_and(|m| (m & 0o170000) == 0o120000);
            let is_dir = file.is_dir()
                || raw_name.ends_with('/')
                || unix_mode.is_some_and(|m| (m & 0o170000) == 0o040000);
            let readonly = unix_mode.is_some_and(|m| (m & 0o222) == 0);

            let len = if is_dir { 0 } else { file.size() };
            let crc32 = if is_dir { None } else { Some(file.crc32()) };

            let modified = file.last_modified().and_then(|dt| {
                // Approximate SystemTime from dos/zip date-time
                let year = dt.year() as u64;
                if year < 1970 {
                    return None;
                }
                let days_approx = year.saturating_sub(1970).saturating_mul(365)
                    + (dt.month() as u64).saturating_mul(30)
                    + (dt.day() as u64);
                let secs = days_approx.saturating_mul(86400)
                    + (dt.hour() as u64).saturating_mul(3600)
                    + (dt.minute() as u64).saturating_mul(60)
                    + (dt.second() as u64);
                UNIX_EPOCH.checked_add(Duration::from_secs(secs))
            });

            let meta = if is_dir {
                VfsMeta::directory(modified)
            } else if is_symlink {
                VfsMeta::symlink(modified)
            } else {
                let mut m = VfsMeta::file(len, modified, crc32);
                m.readonly = readonly;
                m
            };

            if is_dir {
                known_dirs.insert(path.clone());
            }

            // Ensure parent directories exist
            let mut curr = path.parent();
            while let Some(parent) = curr {
                if !known_dirs.contains(&parent) {
                    known_dirs.insert(parent.clone());
                    entries.insert(
                        parent.clone(),
                        ZipItemInfo {
                            meta: VfsMeta::directory(None),
                            zip_index: usize::MAX,
                        },
                    );
                }
                curr = parent.parent();
            }

            entries.insert(
                path,
                ZipItemInfo {
                    meta,
                    zip_index: index,
                },
            );
        }

        // Build child listings for every directory
        for (item_path, item_info) in &entries {
            let parent_path = item_path.parent().unwrap_or_else(VfsPath::root);
            let entry = VfsEntry::new(item_path.clone(), item_info.meta.clone());
            dir_children.entry(parent_path).or_default().push(entry);
        }

        // Sort child entries deterministically
        for list in dir_children.values_mut() {
            list.sort_by(|a, b| a.name.cmp(&b.name));
        }

        Ok(Self {
            data: Arc::new(bytes),
            comment,
            entries,
            dir_children,
        })
    }

    /// Returns the number of distinct items (files and directories) in the archive.
    #[must_use]
    pub fn entries_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the number of regular files in the archive.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.entries.values().filter(|e| e.meta.is_file).count()
    }

    /// Returns the number of directories in the archive.
    #[must_use]
    pub fn dir_count(&self) -> usize {
        self.entries.values().filter(|e| e.meta.is_dir).count()
    }

    /// Returns `true` if the given virtual path exists in the archive.
    #[must_use]
    pub fn contains(&self, path: &VfsPath) -> bool {
        path.is_root() || self.entries.contains_key(path)
    }

    /// Searches for a virtual path matching `query` case-insensitively.
    #[must_use]
    pub fn find_case_insensitive(&self, query: &str) -> Option<VfsPath> {
        let norm = match VfsPath::new(query) {
            Ok(p) => p,
            Err(_) => return None,
        };
        self.entries
            .keys()
            .find(|p| p.as_str().eq_ignore_ascii_case(norm.as_str()))
            .cloned()
    }

    /// Returns a list of all normalized virtual paths present in the archive.
    #[must_use]
    pub fn all_paths(&self) -> Vec<VfsPath> {
        self.entries.keys().cloned().collect()
    }

    /// Returns the raw archive bytes.
    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    /// Returns the total uncompressed bytes of regular files in the archive.
    #[must_use]
    pub fn total_uncompressed_bytes(&self) -> u64 {
        self.entries
            .values()
            .filter(|info| info.meta.is_file)
            .map(|info| info.meta.len)
            .fold(0_u64, u64::saturating_add)
    }

    /// Returns the optional archive comment, if present and valid UTF-8.
    #[must_use]
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Returns a list of all normalized virtual paths present in the archive.
    #[must_use]
    pub fn file_names(&self) -> Vec<VfsPath> {
        self.all_paths()
    }
}

impl Vfs for ZipFs {
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError> {
        if path.is_root() {
            return Ok(self.dir_children.get(path).cloned().unwrap_or_default());
        }

        let item = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if !item.meta.is_dir {
            return Err(VfsError::NotADirectory(path.clone()));
        }

        Ok(self.dir_children.get(path).cloned().unwrap_or_default())
    }

    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError> {
        if path.is_root() {
            return Ok(VfsMeta::directory(None));
        }

        let item = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;
        Ok(item.meta.clone())
    }

    fn read_stream(
        &self,
        path: &VfsPath,
        max_bytes: u64,
    ) -> Result<Box<dyn Read + Send>, VfsError> {
        let item = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if item.meta.is_dir {
            return Err(VfsError::NotAFile(path.clone()));
        }

        if item.meta.len > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: item.meta.len,
            });
        }

        let cursor = Cursor::new(self.data.as_ref());
        let mut archive = ZipArchive::new(cursor).map_err(VfsError::from)?;

        let mut zip_file = archive.by_index(item.zip_index).map_err(VfsError::from)?;
        let mut buffer = Vec::with_capacity(item.meta.len as usize);
        zip_file.read_to_end(&mut buffer).map_err(VfsError::Io)?;

        let bytes_read = buffer.len() as u64;
        if bytes_read > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: bytes_read,
            });
        }

        Ok(Box::new(Cursor::new(buffer)))
    }

    fn write_atomic(&self, _path: &VfsPath, _bytes: &[u8]) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn delete(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
