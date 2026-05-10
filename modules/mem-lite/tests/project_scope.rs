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
