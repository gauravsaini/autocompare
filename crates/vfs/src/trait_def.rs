//! Core Virtual File System (Vfs) trait and metadata types.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::io::Read;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::VfsError;
use crate::path::VfsPath;

/// Metadata describing a node in the virtual filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VfsMeta {
    /// True if the target is a directory.
    pub is_dir: bool,
    /// True if the target is a regular file.
    pub is_file: bool,
    /// True if the target is a symbolic link.
    pub is_symlink: bool,
    /// File size in bytes (0 for directories).
    pub len: u64,
    /// Modification time, when available from the underlying storage.
    pub modified: Option<SystemTime>,
    /// True if write or delete operations on this node are forbidden.
    pub readonly: bool,
    /// Optional pre-computed 32-bit CRC checksum.
    pub crc32: Option<u32>,
}

impl VfsMeta {
    /// Creates directory metadata.
    #[must_use]
    pub const fn directory(modified: Option<SystemTime>) -> Self {
        Self {
            is_dir: true,
            is_file: false,
            is_symlink: false,
            len: 0,
            modified,
            readonly: false,
            crc32: None,
        }
    }

    /// Creates regular file metadata.
    #[must_use]
    pub const fn file(len: u64, modified: Option<SystemTime>, crc32: Option<u32>) -> Self {
        Self {
            is_dir: false,
            is_file: true,
            is_symlink: false,
            len,
            modified,
            readonly: false,
            crc32,
        }
    }

    /// Creates symbolic link metadata.
    #[must_use]
    pub const fn symlink(modified: Option<SystemTime>) -> Self {
        Self {
            is_dir: false,
            is_file: false,
            is_symlink: true,
            len: 0,
            modified,
            readonly: false,
            crc32: None,
        }
    }
}

/// A directory entry describing a child item in the virtual filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VfsEntry {
    /// Virtual path of the entry.
    pub path: VfsPath,
    /// Base name of the file or directory.
    pub name: String,
    /// Metadata associated with the entry.
    pub meta: VfsMeta,
}

impl VfsEntry {
    /// Creates a new directory entry.
    #[must_use]
    pub fn new(path: VfsPath, meta: VfsMeta) -> Self {
        let name = path
            .file_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| "/".to_string());
        Self { path, name, meta }
    }
}

/// Options governing directory traversal and walk behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkOptions {
    /// Whether to follow directory symlinks (default: false).
    pub follow_symlinks: bool,
    /// Maximum recursive depth (0 = root only; None = unbounded).
    pub max_depth: Option<usize>,
    /// Whether to return [`VfsError::SymlinkCycle`] when a cycle is encountered.
    pub refuse_cycles: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            max_depth: None,
            refuse_cycles: true,
        }
    }
}

/// Core abstraction for virtual filesystems, archive readers, and snapshots.
pub trait Vfs: Send + Sync {
    /// Lists all immediate children of the given directory.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if the path does not exist, or
    /// [`VfsError::NotADirectory`] if the path is a regular file.
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError>;

    /// Returns metadata for the given virtual path.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if the path does not exist.
    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError>;

    /// Opens a readable byte stream for the file at `path`, bounded by `max_bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if missing, [`VfsError::NotAFile`] if a directory,
    /// or [`VfsError::FileTooLarge`] if file length exceeds `max_bytes`.
    fn read_stream(&self, path: &VfsPath, max_bytes: u64)
    -> Result<Box<dyn Read + Send>, VfsError>;

    /// Writes data to `path` atomically.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::ReadOnly`] on archives or read-only stores.
    fn write_atomic(&self, path: &VfsPath, bytes: &[u8]) -> Result<(), VfsError>;

    /// Deletes the file or directory at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::ReadOnly`] on archives or read-only stores.
    fn delete(&self, path: &VfsPath) -> Result<(), VfsError>;

    /// Returns `true` if this filesystem cannot be mutated.
    fn is_read_only(&self) -> bool;

    /// Checks if a node exists at `path`.
    fn exists(&self, path: &VfsPath) -> bool {
        self.stat(path).is_ok()
    }

    /// Reads the entire file content into a byte vector, bounded by `max_bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::FileTooLarge`] if bytes exceed `max_bytes`, or
    /// standard I/O errors.
    fn read_all(&self, path: &VfsPath, max_bytes: u64) -> Result<Vec<u8>, VfsError> {
        let meta = self.stat(path)?;
        if meta.is_dir {
            return Err(VfsError::NotAFile(path.clone()));
        }
        if meta.len > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: meta.len,
            });
        }

        let reader = self.read_stream(path, max_bytes)?;
        let mut buffer = Vec::new();
        // Read bounded by max_bytes + 1 to detect over-reads
        let limit = max_bytes.saturating_add(1);
        let mut limited = reader.take(limit);
        limited.read_to_end(&mut buffer)?;

        let bytes_read = buffer.len() as u64;
        if bytes_read > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: bytes_read,
            });
        }

        Ok(buffer)
    }

    /// Recursively walks the directory tree rooted at `root` using breadth-first traversal.
    ///
    /// Symlinks are never descended into when `options.follow_symlinks` is `false`.
    ///
    /// # Errors
    ///
    /// Propagates directory listing errors or symlink cycle errors when encountered.
    fn walk(&self, root: &VfsPath, options: &WalkOptions) -> Result<Vec<VfsEntry>, VfsError> {
        let root_meta = self.stat(root)?;
        if !root_meta.is_dir {
            return Ok(vec![VfsEntry::new(root.clone(), root_meta)]);
        }

        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((root.clone(), 0_usize));

        while let Some((current_dir, current_depth)) = queue.pop_front() {
            if let Some(max) = options.max_depth
                && current_depth >= max
            {
                continue;
            }

            let entries = self.list_dir(&current_dir)?;
            for entry in entries {
                let is_dir = entry.meta.is_dir;
                let is_symlink = entry.meta.is_symlink;
                let child_path = entry.path.clone();
                results.push(entry);

                if is_dir {
                    queue.push_back((child_path, current_depth.saturating_add(1)));
                } else if is_symlink && options.follow_symlinks {
                    // Check if symlink target is a directory
                    if let Ok(target_meta) = self.stat(&child_path)
                        && target_meta.is_dir
                    {
                        queue.push_back((child_path, current_depth.saturating_add(1)));
                    }
                }
            }
        }

        Ok(results)
    }
}
