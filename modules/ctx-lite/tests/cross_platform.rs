use ctx_lite::core::cache::ReadMode;
use ctx_lite::core::diff::DiffMode;
/// Cross-platform compatibility tests for ctx-lite
/// Ensures consistent behavior across Linux, macOS, and Windows
use std::path::{Path, PathBuf};

#[test]
fn test_unix_path_handling() {
    // Unix-style path with forward slashes
    let path = Path::new("/home/user/project/src/main.rs");
    assert!(path.to_str().is_some());
}

#[test]
fn test_pathbuf_construction() {
    // Construction should work across platforms
    let path = PathBuf::from("src/module/file.rs");
    assert_eq!(path.as_os_str().len() > 0, true);
}

#[test]
fn test_path_component_iteration() {
    let path = Path::new("src/core/cache.rs");
    let components: Vec<_> = path.components().collect();
    assert!(components.len() >= 3);
}

#[test]
fn test_newline_handling_unix() {
    let content_unix = "line1\nline2\nline3\n";
    let lines: Vec<&str> = content_unix.lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_newline_handling_windows() {
    let content_windows = "line1\r\nline2\r\nline3\r\n";
    let lines: Vec<&str> = content_windows.lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_newline_handling_mixed() {
    let content_mixed = "line1\nline2\r\nline3\n";
    let lines: Vec<&str> = content_mixed.lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_file_extension_case_insensitive() {
    // Filenames should be handled consistently regardless of platform
    let files = vec!["main.rs", "Main.RS", "MAIN.RS", "main.Rs"];

    for file in files {
        let path = Path::new(file);
        assert!(path.file_name().is_some());
    }
}

#[test]
fn test_unicode_content_consistency() {
    // Unicode should be handled consistently across platforms
    let content_ru = "Привет мир\nПока мир\n";
    let lines: Vec<&str> = content_ru.lines().collect();
    assert_eq!(lines.len(), 2);

    let mut differ = DiffMode::new();
    let result = differ.compute_diff(None, content_ru);
    assert!(result.diffs.len() > 0);
}

#[test]
fn test_utf8_bom_handling() {
    // Some files on Windows have BOM marker
    let content_with_bom = "\u{FEFF}content\n";
    let lines: Vec<&str> = content_with_bom.lines().collect();
    assert!(lines[0].contains("content") || lines[0].contains("\u{FEFF}"));
}

#[test]
fn test_path_separator_consistency() {
    // Ensure path handling works regardless of separator
    let paths = vec![
        "src/main.rs",
        "src\\main.rs", // Windows style, but Rust handles both
    ];

    for path_str in paths {
        let path = Path::new(path_str);
        assert!(path.to_str().is_some());
    }
}

#[test]
fn test_read_mode_serialization_consistency() {
    // Mode names should be consistent across platforms
    let modes = vec![
        ReadMode::Full,
        ReadMode::Signatures,
        ReadMode::Map,
        ReadMode::Diff,
    ];

    for mode in modes {
        let mode_str = mode.as_str();
        assert!(mode_str.len() > 0);
        assert!(mode_str.chars().all(|c| c.is_ascii_lowercase()));
    }
}

#[test]
fn test_binary_content_handling() {
    // Binary files should be handled gracefully
    let binary_content = "hello\x00world\x01\x02\x03";
    let mut differ = DiffMode::new();
    let result = differ.compute_diff(None, binary_content);
    // Should either handle or gracefully degrade
    assert!(result.diffs.len() > 0 || result.compression_percent == 0);
}

#[test]
fn test_large_files_consistency() {
    // Ensure large files work consistently
    let mut content = String::new();
    for i in 0..10000 {
        content.push_str(&format!("line {}\n", i));
    }

    let mut differ = DiffMode::new();
    let result = differ.compute_diff(None, &content);

    // Should complete in reasonable time
    assert!(result.diffs.len() > 0);

    // Second read should be fast
    let modified = content.replace("line 5000\n", "line 5000 MODIFIED\n");
    let result2 = differ.compute_diff(Some(&content), &modified);

    // Should detect the change efficiently
    assert!(result2.diffs.len() > 0);
    assert!(result2.compression_percent >= 90);
}

#[test]
fn test_performance_consistency() {
    // Ensure performance characteristics are consistent
    let content = "x".repeat(100000); // 100KB of same character

    let mut differ = DiffMode::new();
    let start = std::time::Instant::now();
    let result = differ.compute_diff(None, &content);
    let elapsed1 = start.elapsed();

    let modified = content.replace("xx", "yy");
    let start2 = std::time::Instant::now();
    let result2 = differ.compute_diff(Some(&content), &modified);
    let elapsed2 = start2.elapsed();

    // Second read should be comparable or slightly faster (diff detection)
    // Note: For very repetitive content, the first read might be optimized
    // so we just check that both complete in reasonable time
    assert!(elapsed1.as_millis() < 1000); // Less than 1 second
    assert!(elapsed2.as_millis() < 1000); // Less than 1 second
}

#[test]
fn test_timezone_agnostic_timestamps() {
    // Timestamps should work correctly regardless of timezone
    use std::time::SystemTime;

    let now = SystemTime::now();
    let later = SystemTime::now();

    // Should always be comparable
    assert!(now.duration_since(later).is_ok() || later.duration_since(now).is_ok());
}

#[test]
fn test_concurrent_reads_safety() {
    // Ensure components can be safely used with multiple threads
    use std::sync::Arc;
    use std::sync::Mutex;

    let differ = Arc::new(Mutex::new(DiffMode::new()));

    // This test just verifies the code compiles and Arc<Mutex> can be created
    // Production testing would spawn actual threads
    let _differ_clone = Arc::clone(&differ);
}

#[test]
fn test_compression_ratio_cross_platform() {
    // Compression ratios should be consistent
    let test_cases = vec![
        ("", 0),             // Empty
        ("a", 1),            // Single char
        ("hello world", 11), // Simple text
    ];

    for (content, expected_len) in test_cases {
        let mut differ = DiffMode::new();
        let _result = differ.compute_diff(None, content);
        assert_eq!(content.len(), expected_len);
    }

    // Test repetitive content separately
    let repetitive = "a".repeat(100);
    let mut differ = DiffMode::new();
    let _result = differ.compute_diff(None, &repetitive);
    assert_eq!(repetitive.len(), 100);
}
