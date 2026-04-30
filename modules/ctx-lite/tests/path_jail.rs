use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ctx_lite::app::contracts::{ReadRequestNormalized, ServiceErrorKind, TreeRequestNormalized};
use ctx_lite::core::config::AppConfig;
use ctx_lite::core::fs::{FileReader, TreeBuilder};
use ctx_lite::core::security::path_jail::{PathJail, PathJailErrorKind};

static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
fn path_jail_rejects_runtime_root_escape() {
    let fixture = FixtureRepo::new("root-escape");
    let jail = PathJail::from_config(&fixture.config()).expect("fixture config should be valid");

    let error = jail
        .resolve(Path::new("../outside.txt"))
        .expect_err("path jail should reject escaping the configured root");

    assert_eq!(error.kind, PathJailErrorKind::OutsideAllowedRoot);
    assert!(
        error.message.contains("allowed root"),
        "unexpected error message: {}",
        error.message
    );
}

#[test]
fn path_jail_rejects_symlink_escape() {
    let fixture = FixtureRepo::new("symlink-escape");
    if !fixture.supports_file_symlinks() {
        return;
    }
    let jail = PathJail::from_config(&fixture.config()).expect("fixture config should be valid");

    let error = jail
        .resolve(Path::new("links/outside.txt"))
        .expect_err("path jail should reject symlinks that resolve outside the root");

    assert_eq!(error.kind, PathJailErrorKind::SymlinkEscape);
    assert!(
        error.message.contains("symlink"),
        "unexpected error message: {}",
        error.message
    );
}

#[test]
fn resolved_path_rejects_file_swapped_for_outside_symlink_before_open() {
    let fixture = FixtureRepo::new("swap-after-resolve");
    if !fixture.supports_file_symlinks() {
        return;
    }
    let jail = PathJail::from_config(&fixture.config()).expect("fixture config should be valid");
    let resolved = jail
        .resolve(Path::new("README.md"))
        .expect("initial file should resolve inside the root");

    remove_path(&fixture.readme_path());
    if !create_file_symlink(
        fixture.outside_root.join("outside.txt"),
        fixture.readme_path(),
    ) {
        return;
    }

    let error = resolved
        .open_file()
        .expect_err("guarded file open should reject swapped symlinks");

    assert_eq!(error.kind, PathJailErrorKind::SymlinkEscape);
}

#[test]
fn file_reader_reads_files_inside_fixture_repo() {
    let fixture = FixtureRepo::new("read-fixture");
    let reader = FileReader::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let response = reader
        .read(ReadRequestNormalized {
            path: fixture.repo_root.join("README.md"),
            max_bytes: 1024,
        })
        .expect("file reader should read files inside the root");

    assert_eq!(
        response.path,
        fs::canonicalize(fixture.repo_root.join("README.md"))
            .expect("fixture readme should canonicalize")
    );
    assert!(response.content.contains("fixture repository"));
    assert_eq!(response.bytes_read, response.content.len());
    assert!(!response.truncated);
}

#[test]
fn file_reader_rejects_hard_links_to_outside_files() {
    let fixture = FixtureRepo::new("hard-link-read-escape");
    if !fixture.supports_hard_link_policy() {
        return;
    }
    let link_path = fixture.repo_root.join("links/outside-hard.txt");
    create_hard_link(fixture.outside_root.join("outside.txt"), &link_path);
    let reader = FileReader::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let error = reader
        .read(ReadRequestNormalized {
            path: link_path,
            max_bytes: 1024,
        })
        .expect_err("file reader should reject hard links to files outside the root");

    assert_eq!(error.kind, ServiceErrorKind::Unsupported);
    assert!(
        error.message.contains("hard link"),
        "unexpected hard-link error: {}",
        error.message
    );
}

#[test]
fn file_reader_enforces_byte_limits_on_utf8_boundaries() {
    let fixture = FixtureRepo::new("utf8-byte-limit");
    let path = fixture.repo_root.join("unicode.txt");
    fs::write(&path, "ééA").expect("fixture unicode file should be written");
    let reader = FileReader::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let response = reader
        .read(ReadRequestNormalized { path, max_bytes: 3 })
        .expect("reader should return the valid utf-8 prefix within the byte limit");

    assert_eq!(response.content, "é");
    assert_eq!(response.bytes_read, 2);
    assert!(response.truncated);
}

#[test]
fn file_reader_stops_before_invalid_utf8_after_limit() {
    let fixture = FixtureRepo::new("bounded-read-invalid-tail");
    let path = fixture.repo_root.join("bounded.txt");
    fs::write(&path, b"ok\n\xff\xfe").expect("fixture bounded file should be written");
    let reader = FileReader::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let response = reader
        .read(ReadRequestNormalized { path, max_bytes: 3 })
        .expect("reader should not decode bytes past the configured limit");

    assert_eq!(response.content, "ok\n");
    assert_eq!(response.bytes_read, 3);
    assert!(response.truncated);
}

#[test]
fn tree_builder_lists_files_inside_fixture_repo() {
    let fixture = FixtureRepo::new("tree-fixture");
    let tree = TreeBuilder::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let response = tree
        .tree(TreeRequestNormalized {
            path: fixture.repo_root.join("src"),
            max_depth: 3,
            include_hidden: false,
        })
        .expect("tree builder should list files inside the root");

    let root = fs::canonicalize(fixture.repo_root.join("src")).expect("src should canonicalize");
    let listed: BTreeSet<PathBuf> = response
        .entries
        .iter()
        .map(|entry| {
            entry
                .path
                .strip_prefix(&root)
                .expect("tree entry should stay inside the requested root")
                .to_path_buf()
        })
        .collect();

    assert_eq!(response.root, root);
    assert!(listed.contains(Path::new("lib.rs")));
    assert!(listed.contains(Path::new("nested")));
    assert!(listed.contains(Path::new("nested/mod.rs")));
    assert!(
        !listed.contains(Path::new(".hidden")),
        "hidden entries should be excluded when include_hidden is false"
    );
}

#[test]
fn tree_builder_rejects_symlink_escape_during_traversal() {
    let fixture = FixtureRepo::new("tree-symlink-escape");
    if !fixture.supports_file_symlinks() {
        return;
    }
    let tree = TreeBuilder::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let error = tree
        .tree(TreeRequestNormalized {
            path: fixture.repo_root.join("links"),
            max_depth: 2,
            include_hidden: false,
        })
        .expect_err("tree builder should reject symlinks that resolve outside the root");

    assert_eq!(error.kind, ServiceErrorKind::Unsupported);
    assert!(
        error.message.contains("symlink"),
        "unexpected tree error: {}",
        error.message
    );
}

#[test]
fn tree_builder_rejects_hard_links_during_traversal() {
    let fixture = FixtureRepo::new("tree-hard-link-escape");
    if !fixture.supports_hard_link_policy() {
        return;
    }
    create_hard_link(
        fixture.outside_root.join("outside.txt"),
        &fixture.repo_root.join("src/outside-hard.txt"),
    );
    let tree = TreeBuilder::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let error = tree
        .tree(TreeRequestNormalized {
            path: fixture.repo_root.join("src"),
            max_depth: 2,
            include_hidden: false,
        })
        .expect_err("tree builder should reject hard links that could expose outside files");

    assert_eq!(error.kind, ServiceErrorKind::Unsupported);
    assert!(
        error.message.contains("hard link"),
        "unexpected hard-link tree error: {}",
        error.message
    );
}

#[test]
fn tree_builder_rejects_responses_with_too_many_entries() {
    let fixture = FixtureRepo::new("tree-entry-limit");
    fixture.populate_wide_tree("src/too-many", 1_025, "entry");
    let tree = TreeBuilder::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let error = tree
        .tree(TreeRequestNormalized {
            path: fixture.repo_root.join("src/too-many"),
            max_depth: 2,
            include_hidden: false,
        })
        .expect_err("tree builder should reject responses that exceed the entry budget");

    assert_eq!(error.kind, ServiceErrorKind::Unsupported);
    assert!(
        error.message.contains("tree response"),
        "unexpected tree limit error: {}",
        error.message
    );
}

#[test]
fn tree_builder_rejects_responses_that_exceed_the_byte_budget() {
    let fixture = FixtureRepo::new("tree-byte-limit");
    fixture.populate_wide_tree("src/too-large", 400, &"x".repeat(220));
    let tree = TreeBuilder::new(
        PathJail::from_config(&fixture.config()).expect("fixture config should be valid"),
    );

    let error = tree
        .tree(TreeRequestNormalized {
            path: fixture.repo_root.join("src/too-large"),
            max_depth: 2,
            include_hidden: false,
        })
        .expect_err("tree builder should reject responses that exceed the byte budget");

    assert_eq!(error.kind, ServiceErrorKind::Unsupported);
    assert!(
        error.message.contains("byte budget"),
        "unexpected tree byte-limit error: {}",
        error.message
    );
}

#[cfg(windows)]
#[test]
fn path_jail_accepts_windows_absolute_paths_with_different_letter_case() {
    let fixture = FixtureRepo::new("windows-case");
    let jail = PathJail::from_config(&fixture.config()).expect("fixture config should be valid");
    let canonical = fs::canonicalize(fixture.readme_path()).expect("fixture readme should exist");
    let requested = PathBuf::from(invert_ascii_case(&canonical.display().to_string()));

    let resolved = jail
        .resolve(&requested)
        .expect("path jail should accept equivalent absolute paths on windows");

    assert_eq!(resolved.path(), canonical.as_path());
}

struct FixtureRepo {
    root: PathBuf,
    repo_root: PathBuf,
    outside_root: PathBuf,
    symlink_supported: bool,
}

impl FixtureRepo {
    fn new(name: &str) -> Self {
        let fixture_id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("runtime")
            .join(format!("{}-{}-{}", name, std::process::id(), fixture_id));
        let repo_root = root.join("repo");
        let outside_root = root.join("outside");

        if root.exists() {
            remove_path(&root);
        }

        fs::create_dir_all(repo_root.join("src/nested"))
            .expect("fixture src tree should be created");
        fs::create_dir_all(repo_root.join("links")).expect("fixture links dir should be created");
        fs::create_dir_all(repo_root.join(".hidden"))
            .expect("fixture hidden dir should be created");
        fs::create_dir_all(&outside_root).expect("fixture outside dir should be created");

        fs::write(
            repo_root.join("README.md"),
            "fixture repository\nwith readable content\n",
        )
        .expect("fixture readme should be written");
        fs::write(repo_root.join("src/lib.rs"), "pub fn fixture() {}\n")
            .expect("fixture lib should be written");
        fs::write(
            repo_root.join("src/nested/mod.rs"),
            "pub const VALUE: &str = \"fixture\";\n",
        )
        .expect("fixture nested module should be written");
        fs::write(repo_root.join(".hidden/secret.txt"), "hidden\n")
            .expect("fixture hidden file should be written");
        fs::write(outside_root.join("outside.txt"), "outside\n")
            .expect("fixture outside file should be written");

        let symlink_supported = create_file_symlink(
            outside_root.join("outside.txt"),
            repo_root.join("links/outside.txt"),
        );

        Self {
            root,
            repo_root,
            outside_root,
            symlink_supported,
        }
    }

    fn config(&self) -> AppConfig {
        // Canonicalize paths to handle Windows path variations
        let project_root = self
            .repo_root
            .canonicalize()
            .unwrap_or_else(|_| self.repo_root.clone());
        AppConfig {
            project_root: project_root.clone(),
            allowed_roots: vec![project_root],
            ..AppConfig::default()
        }
    }

    fn readme_path(&self) -> PathBuf {
        self.repo_root.join("README.md")
    }

    fn supports_file_symlinks(&self) -> bool {
        self.symlink_supported
    }

    fn supports_hard_link_policy(&self) -> bool {
        cfg!(unix)
    }

    fn populate_wide_tree(&self, relative_dir: &str, count: usize, prefix: &str) {
        let dir = self.repo_root.join(relative_dir);
        fs::create_dir_all(&dir).expect("fixture wide tree directory should be created");

        for index in 0..count {
            fs::write(
                dir.join(format!("{prefix}-{index:04}.txt")),
                format!("fixture {index}\n"),
            )
            .expect("fixture wide tree file should be written");
        }
    }
}

impl Drop for FixtureRepo {
    fn drop(&mut self) {
        if self.root.exists() {
            remove_path(&self.root);
        }
    }
}

fn remove_path(path: &Path) {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).expect("fixture directory should be removed");
        }
        Ok(_) => {
            fs::remove_file(path).expect("fixture file should be removed");
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("failed to inspect fixture path {}: {error}", path.display()),
    }
}

fn create_hard_link(target: PathBuf, link: &Path) {
    fs::hard_link(target, link).expect("fixture hard link should be created");
}

#[cfg(windows)]
fn invert_ascii_case(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() {
                ch.to_ascii_uppercase()
            } else if ch.is_ascii_uppercase() {
                ch.to_ascii_lowercase()
            } else {
                ch
            }
        })
        .collect()
}

#[cfg(unix)]
fn create_file_symlink(target: PathBuf, link: PathBuf) -> bool {
    std::os::unix::fs::symlink(target, link).expect("fixture symlink should be created");
    true
}

#[cfg(windows)]
fn create_file_symlink(target: PathBuf, link: PathBuf) -> bool {
    match std::os::windows::fs::symlink_file(target, link) {
        Ok(()) => true,
        Err(error)
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314) =>
        {
            false
        }
        Err(error) => panic!("fixture symlink should be created: {error}"),
    }
}
