//! Core Virtual File System (Vfs) trait and metadata types.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::io::{self, Cursor, Read};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use crc32fast::Hasher;
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

impl WalkOptions {
    /// Standard traversal options without symlink resolution and unbounded depth.
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            follow_symlinks: false,
            max_depth: None,
            refuse_cycles: true,
        }
    }

    /// Unbounded traversal (alias for `standard`).
    #[must_use]
    pub const fn unbounded() -> Self {
        Self::standard()
    }

    /// Shallow traversal (max depth 1, only immediate children).
    #[must_use]
    pub const fn shallow() -> Self {
        Self {
            follow_symlinks: false,
            max_depth: Some(1),
            refuse_cycles: true,
        }
    }

    /// Bounded traversal up to `max_depth`.
    #[must_use]
    pub const fn bounded(max_depth: usize) -> Self {
        Self {
            follow_symlinks: false,
            max_depth: Some(max_depth),
            refuse_cycles: true,
        }
    }

    /// Sets whether to follow directory symlinks.
    #[must_use]
    pub const fn with_follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Sets the maximum recursive walk depth.
    #[must_use]
    pub const fn with_max_depth(mut self, depth: Option<usize>) -> Self {
        self.max_depth = depth;
        self
    }

    /// Sets whether to refuse symlink cycles with [`VfsError::SymlinkCycle`].
    #[must_use]
    pub const fn with_refuse_cycles(mut self, refuse: bool) -> Self {
        self.refuse_cycles = refuse;
        self
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
        let mut visited_dirs = HashSet::new();
        visited_dirs.insert(root.clone());

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
                    if !visited_dirs.insert(child_path.clone()) {
                        if options.refuse_cycles {
                            return Err(VfsError::SymlinkCycle(child_path));
                        }
                        continue;
                    }
                    queue.push_back((child_path, current_depth.saturating_add(1)));
                } else if is_symlink && options.follow_symlinks {
                    // Check if symlink target is a directory
                    if let Ok(target_meta) = self.stat(&child_path)
                        && target_meta.is_dir
                    {
                        if !visited_dirs.insert(child_path.clone()) {
                            if options.refuse_cycles {
                                return Err(VfsError::SymlinkCycle(child_path));
                            }
                            continue;
                        }
                        queue.push_back((child_path, current_depth.saturating_add(1)));
                    }
                }
            }
        }

        Ok(results)
    }
}

/// Protocol type for remote network virtual filesystems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteKind {
    /// SSH File Transfer Protocol (SFTP).
    Sftp,
    /// Web Distributed Authoring and Versioning (WebDAV).
    WebDav,
    /// Standard File Transfer Protocol (FTP).
    Ftp,
    /// File Transfer Protocol over TLS (FTPS).
    Ftps,
}

impl RemoteKind {
    /// Default standard TCP port for this protocol.
    #[must_use]
    pub const fn default_port(&self) -> u16 {
        match self {
            Self::Sftp => 22,
            Self::WebDav => 443,
            Self::Ftp => 21,
            Self::Ftps => 990,
        }
    }

    /// URL scheme identifier for this protocol.
    #[must_use]
    pub const fn scheme(&self) -> &'static str {
        match self {
            Self::Sftp => "sftp",
            Self::WebDav => "webdav",
            Self::Ftp => "ftp",
            Self::Ftps => "ftps",
        }
    }
}

/// Remote endpoint connection configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEndpoint {
    /// Remote protocol kind.
    pub kind: RemoteKind,
    /// Host name or IP address.
    pub host: String,
    /// TCP port number.
    pub port: u16,
    /// Remote root base path.
    pub base_path: VfsPath,
    /// Username for authentication.
    pub user: Option<String>,
    /// Whether TLS encryption is required.
    pub tls: bool,
}

impl RemoteEndpoint {
    /// Creates a new remote endpoint configuration with the default port.
    #[must_use]
    pub fn new(kind: RemoteKind, host: impl Into<String>, base_path: VfsPath) -> Self {
        let port = kind.default_port();
        let tls = matches!(kind, RemoteKind::WebDav | RemoteKind::Ftps);
        Self {
            kind,
            host: host.into(),
            port,
            base_path,
            user: None,
            tls,
        }
    }

    /// Overrides the default port.
    #[must_use]
    pub const fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the authentication username.
    #[must_use]
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Sets the TLS requirement flag.
    #[must_use]
    pub const fn with_tls(mut self, tls: bool) -> Self {
        self.tls = tls;
        self
    }
}

/// Connection state of a remote network filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteConnectionState {
    /// Session is connected and authenticated.
    Connected,
    /// Session is disconnected.
    Disconnected,
    /// Connection handshake or reconnection is in progress.
    Connecting,
}

/// Trait extension for remote network filesystems (SFTP, WebDAV, FTP).
pub trait RemoteVfs: Vfs {
    /// Returns the remote protocol kind.
    fn protocol(&self) -> RemoteKind;

    /// Returns the endpoint connection parameters.
    fn endpoint(&self) -> &RemoteEndpoint;

    /// Returns the current connection state.
    fn connection_state(&self) -> RemoteConnectionState;

    /// Disconnects the remote connection.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if disconnect negotiation fails.
    fn disconnect(&mut self) -> Result<(), VfsError>;

    /// Reconnects the remote connection.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if connection establishment fails.
    fn reconnect(&mut self) -> Result<(), VfsError>;

    /// Measures round-trip ping latency to the remote host.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if disconnected.
    fn ping(&self) -> Result<Duration, VfsError>;
}

#[derive(Debug)]
struct FakeRemoteStorage {
    state: RemoteConnectionState,
    files: BTreeMap<VfsPath, (VfsMeta, Vec<u8>)>,
    dirs: BTreeSet<VfsPath>,
}

/// An in-memory fake remote filesystem implementing [`Vfs`] and [`RemoteVfs`]
/// for headless unit testing of SFTP and WebDAV profiles without network access.
#[derive(Debug, Clone)]
pub struct FakeRemoteFs {
    endpoint: RemoteEndpoint,
    storage: Arc<RwLock<FakeRemoteStorage>>,
}

impl FakeRemoteFs {
    /// Creates a new in-memory fake remote filesystem starting in the connected state.
    #[must_use]
    pub fn new(endpoint: RemoteEndpoint) -> Self {
        let mut dirs = BTreeSet::new();
        dirs.insert(VfsPath::root());
        let storage = FakeRemoteStorage {
            state: RemoteConnectionState::Connected,
            files: BTreeMap::new(),
            dirs,
        };
        Self {
            endpoint,
            storage: Arc::new(RwLock::new(storage)),
        }
    }

    /// Simulates a network disconnect event.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if lock acquisition fails.
    pub fn simulate_disconnect(&self) -> Result<(), VfsError> {
        let mut storage = self
            .storage
            .write()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;
        storage.state = RemoteConnectionState::Disconnected;
        Ok(())
    }

    /// Simulates a network reconnection event.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if lock acquisition fails.
    pub fn simulate_reconnect(&self) -> Result<(), VfsError> {
        let mut storage = self
            .storage
            .write()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;
        storage.state = RemoteConnectionState::Connected;
        Ok(())
    }

    /// Creates a directory on the fake remote filesystem.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if disconnected or lock poisoned.
    pub fn create_dir_all(&self, path: &VfsPath) -> Result<(), VfsError> {
        let mut storage = self
            .storage
            .write()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;
        if storage.state != RemoteConnectionState::Connected {
            return Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )));
        }

        let mut curr = Some(path.clone());
        while let Some(dir) = curr {
            storage.dirs.insert(dir.clone());
            curr = dir.parent();
        }
        Ok(())
    }
}

impl Vfs for FakeRemoteFs {
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError> {
        let storage = self
            .storage
            .read()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;

        if storage.state != RemoteConnectionState::Connected {
            return Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )));
        }

        if !storage.dirs.contains(path) {
            if storage.files.contains_key(path) {
                return Err(VfsError::NotADirectory(path.clone()));
            }
            return Err(VfsError::NotFound(path.clone()));
        }

        let mut entries = Vec::new();

        for dir in &storage.dirs {
            if dir.is_root() {
                continue;
            }
            if let Some(parent) = dir.parent()
                && parent == *path
            {
                entries.push(VfsEntry::new(dir.clone(), VfsMeta::directory(None)));
            }
        }

        for (file_path, (meta, _)) in &storage.files {
            if let Some(parent) = file_path.parent()
                && parent == *path
            {
                entries.push(VfsEntry::new(file_path.clone(), meta.clone()));
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError> {
        let storage = self
            .storage
            .read()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;

        if storage.state != RemoteConnectionState::Connected {
            return Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )));
        }

        if path.is_root() || storage.dirs.contains(path) {
            return Ok(VfsMeta::directory(None));
        }

        if let Some((meta, _)) = storage.files.get(path) {
            return Ok(meta.clone());
        }

        Err(VfsError::NotFound(path.clone()))
    }

    fn read_stream(
        &self,
        path: &VfsPath,
        max_bytes: u64,
    ) -> Result<Box<dyn Read + Send>, VfsError> {
        let storage = self
            .storage
            .read()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;

        if storage.state != RemoteConnectionState::Connected {
            return Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )));
        }

        if path.is_root() || storage.dirs.contains(path) {
            return Err(VfsError::NotAFile(path.clone()));
        }

        let (meta, bytes) = storage
            .files
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if meta.len > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: meta.len,
            });
        }

        Ok(Box::new(Cursor::new(bytes.clone())))
    }

    fn write_atomic(&self, path: &VfsPath, bytes: &[u8]) -> Result<(), VfsError> {
        let mut storage = self
            .storage
            .write()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;

        if storage.state != RemoteConnectionState::Connected {
            return Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )));
        }

        if path.is_root() || storage.dirs.contains(path) {
            return Err(VfsError::NotAFile(path.clone()));
        }

        let mut curr = path.parent();
        while let Some(parent) = curr {
            storage.dirs.insert(parent.clone());
            curr = parent.parent();
        }

        let mut hasher = Hasher::new();
        hasher.update(bytes);
        let crc = hasher.finalize();

        let meta = VfsMeta::file(bytes.len() as u64, Some(SystemTime::now()), Some(crc));
        storage.files.insert(path.clone(), (meta, bytes.to_vec()));
        Ok(())
    }

    fn delete(&self, path: &VfsPath) -> Result<(), VfsError> {
        let mut storage = self
            .storage
            .write()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;

        if storage.state != RemoteConnectionState::Connected {
            return Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )));
        }

        if path.is_root() {
            return Err(VfsError::PermissionDenied(path.clone()));
        }

        let file_removed = storage.files.remove(path).is_some();
        let dir_removed = storage.dirs.remove(path);

        if !file_removed && !dir_removed {
            return Err(VfsError::NotFound(path.clone()));
        }

        Ok(())
    }

    fn is_read_only(&self) -> bool {
        false
    }
}

impl RemoteVfs for FakeRemoteFs {
    fn protocol(&self) -> RemoteKind {
        self.endpoint.kind
    }

    fn endpoint(&self) -> &RemoteEndpoint {
        &self.endpoint
    }

    fn connection_state(&self) -> RemoteConnectionState {
        self.storage
            .read()
            .map(|s| s.state)
            .unwrap_or(RemoteConnectionState::Disconnected)
    }

    fn disconnect(&mut self) -> Result<(), VfsError> {
        self.simulate_disconnect()
    }

    fn reconnect(&mut self) -> Result<(), VfsError> {
        self.simulate_reconnect()
    }

    fn ping(&self) -> Result<Duration, VfsError> {
        let storage = self
            .storage
            .read()
            .map_err(|_| VfsError::Io(io::Error::other("remote storage lock poisoned")))?;
        if storage.state == RemoteConnectionState::Connected {
            Ok(Duration::from_millis(1))
        } else {
            Err(VfsError::Io(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote host is disconnected",
            )))
        }
    }
}
