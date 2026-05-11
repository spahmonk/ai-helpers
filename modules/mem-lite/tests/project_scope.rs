use mem_lite::ProjectScope;
use tempfile::tempdir;

#[test]
fn different_workspace_roots_get_different_project_fingerprints() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();

    let left_scope = ProjectScope::from_workspace_root(left.path()).unwrap();
    let right_scope = ProjectScope::from_workspace_root(right.path()).unwrap();

    assert_ne!(left_scope.project_id, right_scope.project_id);
    assert_ne!(left_scope.database_path, right_scope.database_path);
}

#[cfg(target_os = "linux")]
#[test]
fn invalid_utf8_workspace_roots_get_distinct_project_ids() {
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::ffi::OsStringExt;

    let temp = tempdir().unwrap();
    let left = temp.path().join(OsString::from_vec(vec![0x80]));
    let right = temp.path().join(OsString::from_vec(vec![0x81]));

    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();

    let left_scope = ProjectScope::from_workspace_root(&left).unwrap();
    let right_scope = ProjectScope::from_workspace_root(&right).unwrap();

    assert_ne!(left_scope.project_id, right_scope.project_id);
}

#[test]
fn same_workspace_root_produces_stable_project_identity() {
    let temp = tempdir().unwrap();
    let first = ProjectScope::from_workspace_root(temp.path()).unwrap();
    let second = ProjectScope::from_workspace_root(temp.path()).unwrap();

    assert_eq!(first.project_id, second.project_id);
    assert_eq!(first.database_path, second.database_path);
}

#[test]
fn relative_workspace_root_is_rejected() {
    let error = ProjectScope::from_workspace_root(std::path::Path::new("relative"))
        .expect_err("relative workspace roots should be rejected");

    assert!(error.to_string().contains("absolute"));
}
