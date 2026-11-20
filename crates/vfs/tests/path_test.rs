//! Hostile [`VfsPath`] inputs: traversal, null bytes, absolute and
//! Windows-style paths must become structured [`VfsError`] values or
//! safe normalizations, never panics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use autocompare_vfs::{VfsError, VfsPath};

#[test]
fn hostile_parent_traversal_is_rejected() {
    for raw in ["..", "../evil", "a/../../evil", "a/..", "..\\evil"] {
        let err = VfsPath::new(raw).expect_err("traversal must fail");
        assert!(
            matches!(err, VfsError::InvalidPath(_)),
            "unexpected error for {raw:?}: {err}"
        );
    }
}

#[test]
fn hostile_null_byte_is_rejected() {
    let err = VfsPath::new("a\0b").expect_err("null byte must fail");
    assert!(matches!(err, VfsError::InvalidPath(_)));
}

#[test]
fn hostile_join_never_escapes() {
    let root = VfsPath::root();
    for child in ["../evil", "..", "a/../../b", ""] {
        if child.is_empty() {
            assert_eq!(root.join(child).expect("empty joins to self"), root);
        } else {
            assert!(root.join(child).is_err(), "join must reject {child:?}");
        }
    }
    // Absolute and backslash inputs normalize inside the virtual root.
    assert_eq!(
        VfsPath::new("/abs/path")
            .expect("absolute normalizes")
            .as_str(),
        "abs/path"
    );
    assert_eq!(
        VfsPath::new("a\\b").expect("backslash normalizes").as_str(),
        "a/b"
    );
}

#[test]
fn hostile_depth_and_prefix_helpers_stay_total() {
    let deep = VfsPath::new("a/b/c").expect("valid path");
    assert_eq!(deep.depth(), 3);
    assert_eq!(VfsPath::root().depth(), 0);
    let base = VfsPath::new("a").expect("valid base");
    assert!(deep.starts_with(&base));
    assert!(!base.starts_with(&deep));
    // A sibling prefix is not a prefix: "ab" is not under "a".
    let sibling = VfsPath::new("ab").expect("valid sibling");
    assert!(!sibling.starts_with(&base));
    assert_eq!(
        deep.strip_prefix(&base).expect("prefix strips").as_str(),
        "b/c"
    );
    assert!(sibling.strip_prefix(&base).is_none());
}
