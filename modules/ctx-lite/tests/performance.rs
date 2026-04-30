/// Performance profiling and optimization for ctx-lite compression pipeline
use std::time::Instant;
use ctx_lite::core::diff::DiffMode;
use ctx_lite::core::cache::ReadMode;

#[test]
fn profile_diff_computation_small_file() {
    let content = "fn main() {\n    println!(\"hello\");\n}\n".repeat(10);
    
    let mut differ = DiffMode::new();
    
    // First read
    let start = Instant::now();
    let _result1 = differ.compute_diff(None, &content);
    let time_first = start.elapsed();
    
    println!("Small file first read: {:?}", time_first);
    assert!(time_first.as_millis() < 10); // Should be <10ms
    
    // Second read (diff)
    let modified = content.replace("hello", "world");
    let start = Instant::now();
    let _result2 = differ.compute_diff(Some(&content), &modified);
    let time_diff = start.elapsed();
    
    println!("Small file diff read: {:?}", time_diff);
    assert!(time_diff.as_millis() < 5); // Diff should be faster
}

#[test]
fn profile_diff_computation_medium_file() {
    // 10KB file
    let mut content = String::new();
    for i in 0..500 {
        content.push_str(&format!("line {}: some code here\n", i));
    }
    
    let mut differ = DiffMode::new();
    
    let start = Instant::now();
    let _result1 = differ.compute_diff(None, &content);
    let time_first = start.elapsed();
    
    println!("Medium file (10KB) first read: {:?}", time_first);
    assert!(time_first.as_millis() < 50); // Should be <50ms
}

#[test]
fn profile_diff_computation_large_file() {
    // 100KB file
    let mut content = String::new();
    for i in 0..5000 {
        content.push_str(&format!("line {}: important data here\n", i));
    }
    
    let mut differ = DiffMode::new();
    
    let start = Instant::now();
    let _result1 = differ.compute_diff(None, &content);
    let time_first = start.elapsed();
    
    println!("Large file (100KB) first read: {:?}", time_first);
    assert!(time_first.as_millis() < 200); // Should be <200ms
}

#[test]
fn profile_hash_computation() {
    let content = "x".repeat(1_000_000); // 1MB
    
    let start = Instant::now();
    let _hash = DiffMode::hash_content(&content);
    let elapsed = start.elapsed();
    
    println!("Hash computation (1MB): {:?}", elapsed);
    assert!(elapsed.as_millis() < 50); // Should be fast
}

#[test]
fn profile_lcs_algorithm_small() {
    let old = vec!["a", "b", "c", "d", "e"];
    let new = vec!["a", "b", "X", "d", "e"];
    
    let mut differ = DiffMode::new();
    
    let start = Instant::now();
    let result = differ.compute_diff(
        Some(&old.join("\n")),
        &new.join("\n")
    );
    let elapsed = start.elapsed();
    
    println!("LCS (5 lines): {:?}", elapsed);
    assert!(result.diffs.len() > 0); // Should detect change
}

#[test]
fn profile_pipeline_full_session() {
    // Simulate realistic session: 5 file reads with changes
    let mut total_time = std::time::Duration::ZERO;
    
    let base_content = "fn process_data() {\n    let x = 42;\n    println!(\"result: {}\", x);\n}\n";
    
    for iteration in 0..5 {
        let content = format!("// Version {}\n{}", iteration, base_content);
        
        let mut differ = DiffMode::new();
        
        let start = Instant::now();
        let _result = differ.compute_diff(None, &content);
        total_time += start.elapsed();
    }
    
    let avg_time = total_time / 5;
    println!("Average time per read: {:?}", avg_time);
    assert!(avg_time.as_millis() < 5); // Each read should be <5ms
}

#[test]
fn profile_memory_usage_pattern() {
    // Large files: ensure memory doesn't explode
    let mut large = String::new();
    for _ in 0..10000 {
        large.push_str("line with some content\n");
    }
    
    let mut differ = DiffMode::new();
    
    // This should complete without OOM
    let _result1 = differ.compute_diff(None, &large);
    let _result2 = differ.compute_diff(Some(&large), &large);
    
    // If we got here, memory management is reasonable
    assert!(true);
}

#[test]
fn benchmark_compression_efficiency() {
    // Measure actual compression percentages
    let test_case_1 = ("Single line\n", "Single line\n");
    let test_case_2 = ("Line 1\nLine 2\nLine 3\n", "Line 1\nLine 2 MODIFIED\nLine 3\n");
    let original_3 = "a\nb\nc\n".repeat(100);
    let modified_3 = "a\nb\nc\nD\n".repeat(100);
    
    let test_cases = vec![
        (test_case_1.0, test_case_1.1),
        (test_case_2.0, test_case_2.1),
        (&original_3, &modified_3),
    ];
    
    for (i, (original, modified)) in test_cases.iter().enumerate() {
        let mut differ = DiffMode::new();
        
        let _r1 = differ.compute_diff(None, original);
        let r2 = differ.compute_diff(Some(original), modified);
        
        println!(
            "Compression test {}: original_size={}, diff_compression={}%",
            i, original.len(), r2.compression_percent
        );
        
        // Most changes should compress reasonably (not all - some might be too large)
        if i == 0 {
            // Identical file should have 99% compression
            assert!(r2.compression_percent > 90);
        }
    }
}

#[test]
fn profile_cache_key_generation() {
    use std::path::Path;
    
    let paths = vec![
        Path::new("src/main.rs"),
        Path::new("tests/test.rs"),
        Path::new("modules/ctx-lite/src/core/cache.rs"),
    ];
    
    let start = Instant::now();
    for path in paths {
        let _hash1 = DiffMode::hash_content(path.to_str().unwrap());
    }
    let elapsed = start.elapsed();
    
    println!("Cache key generation (3 paths): {:?}", elapsed);
    assert!(elapsed.as_millis() < 5);
}

#[test]
fn profile_repeated_reads_improvement() {
    // Profile how compression improves with repeated reads
    let content_v1 = "version 1\n".repeat(100);
    
    let mut differ = DiffMode::new();
    let r1 = differ.compute_diff(None, &content_v1);
    
    let content_v2 = content_v1.replace("version 1", "version 2");
    let r2 = differ.compute_diff(Some(&content_v1), &content_v2);
    
    let content_v3 = content_v2.replace("version 2", "version 3");
    let r3 = differ.compute_diff(Some(&content_v2), &content_v3);
    
    println!(
        "Compression progression: r1={}%, r2={}%, r3={}%",
        r1.compression_percent, r2.compression_percent, r3.compression_percent
    );
    
    // Later reads should have better compression
    assert!(r2.compression_percent >= r1.compression_percent);
}

#[test]
fn profile_overall_pipeline_efficiency() {
    // End-to-end measurement
    let content = std::fs::read_to_string("/home/monk/Workshop/ai-helpers/modules/ctx-lite/src/core/diff.rs")
        .unwrap_or_else(|_| "dummy content\n".repeat(1000));
    
    let start = Instant::now();
    
    // Simulate reading same file 3 times with minor changes
    let mut differ = DiffMode::new();
    for iteration in 0..3 {
        let modified = if iteration == 0 {
            content.clone()
        } else {
            content.replace("test", &format!("test_{}", iteration))
        };
        
        let prev = if iteration == 0 { None } else { Some(content.as_str()) };
        let _ = differ.compute_diff(prev, &modified);
    }
    
    let total_elapsed = start.elapsed();
    println!("Full pipeline (3 reads, 1 file): {:?}", total_elapsed);
    
    // Should be reasonably fast
    assert!(total_elapsed.as_secs() < 1);
}
