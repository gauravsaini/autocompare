//! Normalized virtual filesystem paths.

#![forbid(unsafe_code)]

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::VfsError;

/// A normalized, relative, forward-slash separated virtual path.
///
/// An empty path `""` represents the root of the virtual filesystem.
/// A path never contains leading or trailing slashes, empty segments,
/// dot segments (`.`), or parent traversal segments (`..`).
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VfsPath(String);

impl VfsPath {
    /// Creates a path representing the VFS root.
    #[must_use]
    pub const fn root() -> Self {
        Self(String::new())
    }

    /// Normalizes and parses a string slice into a [`VfsPath`].
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if the path contains null bytes
    /// or parent-traversal components (`..`).
    pub fn new(raw: impl AsRef<str>) -> Result<Self, VfsError> {
        let s = raw.as_ref();
        if s.contains('\0') {
            return Err(VfsError::InvalidPath("path contains null byte".to_string()));
        }

        let mut parts = Vec::new();
        let normalized = s.replace('\\', "/");
        for component in normalized.split('/') {
            let trimmed = component.trim();
            if trimmed.is_empty() || trimmed == "." {
                continue;
            }
            if trimmed == ".." {
                return Err(VfsError::InvalidPath(
                    "parent directory traversal '..' forbidden in VfsPath".to_string(),
                ));
            }
            parts.push(trimmed.to_string());
        }

        Ok(Self(parts.join("/")))
    }

    /// Creates a [`VfsPath`] from a local filesystem path relative to a base directory.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if relative path resolution fails or
    /// contains invalid components.
    pub fn from_relative_path(path: &Path) -> Result<Self, VfsError> {
        let str_rep = path
            .to_str()
            .ok_or_else(|| VfsError::InvalidPath("non-UTF8 path".to_string()))?;
        Self::new(str_rep)
    }

    /// Returns `true` if this path refers to the root of the virtual filesystem.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if this path is empty (refers to root).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.is_root()
    }

    /// Returns the normalized path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the immediate parent path, or `None` if this path is already root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            return None;
        }

        match self.0.rfind('/') {
            Some(idx) => {
                let parent_slice = self.0.get(..idx).unwrap_or("");
                Some(Self(parent_slice.to_string()))
            }
            None => Some(Self::root()),
        }
    }

    /// Returns the final file or directory name component of the path.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        if self.is_root() {
            return None;
        }

        match self.0.rfind('/') {
            Some(idx) => {
                let start = idx.saturating_add(1);
                self.0.get(start..)
            }
            None => Some(&self.0),
        }
    }

    /// Returns the extension of the file, if present.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        let name = self.file_name()?;
        let dot_idx = name.rfind('.')?;
        if dot_idx == 0 {
            return None; // Hidden dotfile without extension
        }
        let ext_start = dot_idx.saturating_add(1);
        name.get(ext_start..)
    }

    /// Returns `true` if this path has the given file extension (case-insensitive).
    #[must_use]
    pub fn has_extension(&self, ext: &str) -> bool {
        self.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext.trim_start_matches('.')))
    }

    /// Returns a new path with the file name replaced.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if this path is root or the new file name is invalid.
    pub fn with_file_name(&self, name: &str) -> Result<Self, VfsError> {
        if self.is_root() {
            return Err(VfsError::InvalidPath(
                "cannot set file name on root path".to_string(),
            ));
        }
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains('\0')
            || name == ".."
            || name == "."
        {
            return Err(VfsError::InvalidPath(format!("invalid file name: {name}")));
        }
        match self.parent() {
            Some(parent) if !parent.is_root() => parent.join(name),
            _ => Self::new(name),
        }
    }

    /// Returns the stem of the file (name without extension).
    #[must_use]
    pub fn file_stem(&self) -> Option<&str> {
        let name = self.file_name()?;
        match name.rfind('.') {
            Some(dot_idx) if dot_idx > 0 => name.get(..dot_idx),
            _ => Some(name),
        }
    }

    /// Strips the given base prefix from this path, returning the relative child path.
    #[must_use]
    pub fn strip_prefix(&self, base: &Self) -> Option<Self> {
        if base.is_root() {
            return Some(self.clone());
        }
        if self == base {
            return Some(Self::root());
        }

        let base_len = base.0.len();
        let remainder = self.0.get(base_len..)?;
        let trimmed = remainder.strip_prefix('/')?;
        Self::new(trimmed).ok()
    }

    /// Returns `true` if this path starts with the given base path.
    #[must_use]
    pub fn starts_with(&self, base: &Self) -> bool {
        if base.is_root() {
            return true;
        }
        if self == base {
            return true;
        }
        if let Some(remainder) = self.0.get(base.0.len()..) {
            remainder.starts_with('/')
        } else {
            false
        }
    }

    /// Returns `true` if this path string ends with the given suffix.
    #[must_use]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }

    /// Returns a new path with the given extension replaced or appended.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if this path is root.
    pub fn with_extension(&self, ext: &str) -> Result<Self, VfsError> {
        if self.is_root() {
            return Err(VfsError::InvalidPath(
                "cannot set extension on root path".to_string(),
            ));
        }

        let stem = self
            .file_stem()
            .ok_or_else(|| VfsError::InvalidPath("missing file stem".to_string()))?;

        let new_file_name = if ext.is_empty() {
            stem.to_string()
        } else {
            format!("{stem}.{ext}")
        };

        match self.parent() {
            Some(parent) if !parent.is_root() => parent.join(&new_file_name),
            _ => Self::new(&new_file_name),
        }
    }

    /// Joins a child path to this path, validating the resulting path.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if the child contains traversal components.
    pub fn join(&self, child: &str) -> Result<Self, VfsError> {
        let child_path = Self::new(child)?;
        if child_path.is_root() {
            return Ok(self.clone());
        }
        if self.is_root() {
            return Ok(child_path);
        }

        let mut combined = String::with_capacity(self.0.len() + 1 + child_path.0.len());
        combined.push_str(&self.0);
        combined.push('/');
        combined.push_str(&child_path.0);
        Ok(Self(combined))
    }

    /// Returns the directory depth (root has depth 0).
    #[must_use]
    pub fn depth(&self) -> usize {
        if self.is_root() {
            0
        } else {
            self.0.split('/').count()
        }
    }

    /// Returns an iterator over each path component.
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|s| !s.is_empty())
    }

    /// Converts this virtual path to a [`PathBuf`] under a base physical directory.
    #[must_use]
    pub fn to_physical_path(&self, base_dir: &Path) -> PathBuf {
        let mut result = base_dir.to_path_buf();
        for component in self.components() {
            result.push(component);
        }
        result
    }

    /// Returns an iterator yielding this path and each of its ancestors up to root.
    #[must_use]
    pub fn ancestors(&self) -> VfsPathAncestors {
        VfsPathAncestors {
            current: Some(self.clone()),
        }
    }
}

/// An iterator over [`VfsPath`] and its ancestors up to root.
#[derive(Debug, Clone)]
pub struct VfsPathAncestors {
    current: Option<VfsPath>,
}

impl Iterator for VfsPathAncestors {
    type Item = VfsPath;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.current.take()?;
        if !curr.is_root() {
            self.current = curr.parent();
        }
        Some(curr)
    }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_root() {
            write!(f, "/")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl fmt::Debug for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VfsPath(\"{}\")", self)
    }
}

impl AsRef<str> for VfsPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::str::FromStr for VfsPath {
    type Err = VfsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
