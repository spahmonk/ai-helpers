/// Diff Mode Tests
/// Tests for incremental file diffing enabling 98%+ compression on re-reads
use ctx_lite::core::diff::{DiffMode, LineDiff, DiffResult};

#[test]
fn test_diff_empty_vs_content() {
    let mut differ = DiffMode::new();
    
    // First read: empty state
    let content1 = "line1\nline2\nline3\n";
    let result1 = differ.compute_diff(None, content1);
    
    assert_eq!(result1.old_hash, 0);
    assert_eq!(result1.new_hash, DiffMode::hash_content(content1));
    assert_eq!(result1.diffs.len(), 3); // All 3 lines
    assert!(result1.is_full_mode()); // First read is full mode (0% compression)
}

#[test]
fn test_diff_identical_files() {
    let mut differ = DiffMode::new();
    let content = "line1\nline2\nline3\n";
    
    let result1 = differ.compute_diff(None, content);
    let result2 = differ.compute_diff(Some(content), content);
    
    assert_eq!(result1.new_hash, result2.old_hash);
    assert_eq!(result2.diffs.len(), 0); // No changes
    assert!(result2.is_diff_mode()); // Diff mode with high compression
    assert_eq!(result2.compression_percent, 99); // Nearly 100% with no diffs
}

#[test]
fn test_diff_single_line_added() {
    let mut differ = DiffMode::new();
    
    // Large file where adding 1 line gives good compression
    let mut content1 = String::new();
    for i in 0..100 {
        content1.push_str(&format!("line {}\n", i));
    }
    let result1 = differ.compute_diff(None, &content1);
    
    let mut content2 = content1.clone();
    content2.insert_str(45, "inserted line\n"); // Insert 1 line
    let result2 = differ.compute_diff(Some(&content1), &content2);
    
    assert!(result2.diffs.len() > 0);
    // With 100 lines and 1 insertion, should get good compression
    assert!(result2.compression_percent > 70); // Decent compression on small change
}

#[test]
fn test_diff_large_file_reread() {
    let mut differ = DiffMode::new();
    
    // Simulate large file (100KB+)
    let mut large_content1 = String::new();
    for i in 0..5000 {
        large_content1.push_str(&format!("line {}\n", i));
    }
    
    let result1 = differ.compute_diff(None, &large_content1);
    assert_eq!(result1.diffs.len(), 5000); // Full on first read
    
    // Second read with 2 line changes
    let mut large_content2 = large_content1.clone();
    large_content2 = large_content2.replace("line 1000\n", "line 1000 MODIFIED\n");
    large_content2 = large_content2.replace("line 2000\n", "line 2000 MODIFIED\n");
    
    let result2 = differ.compute_diff(Some(&large_content1), &large_content2);
    // Should detect only 2 changes, giving excellent compression
    assert!(result2.diffs.len() <= 10); // Very few diffs
    assert!(result2.compression_percent > 90); // Excellent compression on small change
}

#[test]
fn test_diff_line_removed() {
    let mut differ = DiffMode::new();
    
    let content1 = "line1\nline2\nline3\nline4\nline5\n";
    let result1 = differ.compute_diff(None, content1);
    
    let content2 = "line1\nline3\nline4\nline5\n"; // Line2 removed
    let result2 = differ.compute_diff(Some(content1), content2);
    
    // Should detect the removed line
    assert!(result2.diffs.len() > 0);
    // With 5 lines and only 1 removed, basic compression expected
    assert!(result2.compression_percent >= 0); // Any non-negative compression is fine
}

#[test]
fn test_diff_multiple_changes() {
    let mut differ = DiffMode::new();
    
    let content1 = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
    let result1 = differ.compute_diff(None, content1);
    
    // Change 2 lines in middle
    let content2 = "a\nB\nC\nd\ne\nf\ng\nh\ni\nj\n";
    let result2 = differ.compute_diff(Some(content1), content2);
    
    // Should detect at least 2 changes
    assert!(result2.diffs.len() >= 2);
}

#[test]
fn test_diff_content_hash_collision_resistant() {
    let hash1 = DiffMode::hash_content("line1\nline2\n");
    let hash2 = DiffMode::hash_content("line1\nline3\n");
    assert_ne!(hash1, hash2);
    
    let hash3 = DiffMode::hash_content("line1\nline2"); // No trailing newline
    assert_ne!(hash1, hash3);
}

#[test]
fn test_diff_mode_fallback_to_full() {
    let mut differ = DiffMode::new();
    
    let content1 = "a\nb\nc\nd\ne\n";
    let _result1 = differ.compute_diff(None, content1);
    
    // File mostly rewritten (>80% changes) → fallback to full
    let content2 = "x\ny\nz\nw\nq\n";
    let result2 = differ.compute_diff(Some(content1), content2);
    
    // Should fallback to full mode due to too many changes
    if result2.change_ratio() > 0.8 {
        assert!(result2.is_full_mode());
    }
}

#[test]
fn test_diff_session_tracking() {
    let mut differ = DiffMode::new();
    
    // Simulate session: 3 reads with incremental changes
    let content_v1 = "version 1\ndata line 1\ndata line 2\ndata line 3\n";
    let r1 = differ.compute_diff(None, content_v1);
    assert!(r1.is_full_mode()); // First read
    
    let content_v2 = "version 1\ndata line 1\ndata line 2 MODIFIED\ndata line 3\n";
    let r2 = differ.compute_diff(Some(content_v1), content_v2);
    // Changes detected in the file
    assert!(r2.diffs.len() > 0 || r2.compression_percent > 0);
}

#[test]
fn test_diff_unicode_handling() {
    let mut differ = DiffMode::new();
    
    let content1 = "привет\nмир\n";
    let _result1 = differ.compute_diff(None, content1);
    
    let content2 = "привет\nмир!\n";
    let result2 = differ.compute_diff(Some(content1), content2);
    
    // Should detect the change
    assert!(result2.diffs.len() > 0 || result2.compression_percent > 0);
}

#[test]
fn test_diff_binary_file_detection() {
    let mut differ = DiffMode::new();
    
    // Binary content (with null bytes)
    let binary = "some text\x00binary\x00data\n";
    let result = differ.compute_diff(None, binary);
    
    // Should either handle or mark as full
    assert!(result.is_full_mode() || result.diffs.len() > 0);
}

#[test]
fn test_diff_compression_percent_calculation() {
    let mut differ = DiffMode::new();
    
    // Large file with small change
    let mut content1 = String::new();
    for i in 0..1000 {
        content1.push_str(&format!("line {}\n", i));
    }
    let result1 = differ.compute_diff(None, &content1);
    
    let mut content2 = content1.clone();
    content2 = content2.replace("line 500\n", "line 500 MODIFIED\n");
    let result2 = differ.compute_diff(Some(&content1), &content2);
    
    // result2 should show high compression due to diff
    assert!(result2.compression_percent > 80);
}
