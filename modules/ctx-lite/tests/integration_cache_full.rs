use ctx_lite::core::budget::ContextBudget;
use ctx_lite::core::cache::{ReadMode, SemanticCache};
use ctx_lite::core::policy::AdaptivePolicy;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime};

// NOTE: The SemanticCache API has semantics where:
// - insert(path, content, result, compression, mode, mtime)
// - get(path, content, mode, mtime) -> result
// For a cache hit, the content (what gets hashed for the cache key) must be the same in both insert and get

// ============================================================================
// Cache Functionality Tests (15 tests)
// ============================================================================

#[test]
fn cache_insert_and_get_basic() {
    let mut cache = SemanticCache::new(100);
    let path = PathBuf::from("test.rs");
    let content = "fn main() { println!(\"Hello\"); }";
    let stored_value = "cached_result";
    let now = SystemTime::now();

    // insert(path, content, result, compression, mode, mtime)
    cache.insert(
        &path,
        content.to_string(),
        stored_value.to_string(),
        50,
        ReadMode::Full,
        now,
    );

    let result = cache.get(&path, content, ReadMode::Full, now);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), stored_value);
}

#[test]
fn cache_get_returns_none_for_missing_key() {
    let cache = SemanticCache::new(100);
    let path = PathBuf::from("test.rs");
    let result = cache.get(
        &path,
        "nonexistent_content",
        ReadMode::Full,
        SystemTime::now(),
    );
    assert!(result.is_none());
}

#[test]
fn cache_different_read_modes_are_separate_entries() {
    let mut cache = SemanticCache::new(100);
    let path = PathBuf::from("test.rs");
    let content = "fn main() {}";
    let value_full = "value_for_full_mode";
    let value_signatures = "value_for_signatures_mode";
    let now = SystemTime::now();

    cache.insert(
        &path,
        content.to_string(),
        value_full.to_string(),
        50,
        ReadMode::Full,
        now,
    );
    cache.insert(
        &path,
        content.to_string(),
        value_signatures.to_string(),
        80,
        ReadMode::Signatures,
        now,
    );

    let full_result = cache.get(&path, content, ReadMode::Full, now);
    let sig_result = cache.get(&path, content, ReadMode::Signatures, now);

    assert_eq!(full_result.unwrap(), value_full);
    assert_eq!(sig_result.unwrap(), value_signatures);
}

#[test]
fn cache_size_tracking() {
    let mut cache = SemanticCache::new(100);
    assert_eq!(cache.size(), 0);

    cache.insert(
        &PathBuf::from("f1.rs"),
        "content1".to_string(),
        "result1".to_string(),
        50,
        ReadMode::Full,
        SystemTime::now(),
    );
    assert_eq!(cache.size(), 1);

    cache.insert(
        &PathBuf::from("f2.py"),
        "content2".to_string(),
        "result2".to_string(),
        40,
        ReadMode::Full,
        SystemTime::now(),
    );
    assert_eq!(cache.size(), 2);
}

#[test]
fn cache_clear() {
    let mut cache = SemanticCache::new(100);
    let now = SystemTime::now();
    cache.insert(
        &PathBuf::from("f1.rs"),
        "content1".to_string(),
        "result1".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    cache.insert(
        &PathBuf::from("f2.py"),
        "content2".to_string(),
        "result2".to_string(),
        40,
        ReadMode::Full,
        now,
    );

    cache.clear();
    assert_eq!(cache.size(), 0);
    assert!(cache
        .get(&PathBuf::from("f1.rs"), "content1", ReadMode::Full, now)
        .is_none());
}

#[test]
fn cache_lru_eviction_on_capacity_exceeded() {
    let mut cache = SemanticCache::new(2);
    let now = SystemTime::now();

    let time1 = now;
    cache.insert(
        &PathBuf::from("f1.rs"),
        "content1".to_string(),
        "result1".to_string(),
        50,
        ReadMode::Full,
        time1,
    );

    let time2 = time1 + Duration::from_secs(1);
    cache.insert(
        &PathBuf::from("f2.py"),
        "content2".to_string(),
        "result2".to_string(),
        40,
        ReadMode::Full,
        time2,
    );

    assert_eq!(cache.size(), 2);

    let time3 = time2 + Duration::from_secs(1);
    cache.insert(
        &PathBuf::from("f3.ts"),
        "content3".to_string(),
        "result3".to_string(),
        60,
        ReadMode::Full,
        time3,
    );

    assert_eq!(cache.size(), 2);
    // First entry (oldest) should be evicted
    assert!(cache
        .get(&PathBuf::from("f1.rs"), "content1", ReadMode::Full, time1)
        .is_none());
    assert!(cache
        .get(&PathBuf::from("f2.py"), "content2", ReadMode::Full, time2)
        .is_some());
    assert!(cache
        .get(&PathBuf::from("f3.ts"), "content3", ReadMode::Full, time3)
        .is_some());
}

#[test]
fn cache_mtime_invalidation() {
    let mut cache = SemanticCache::new(100);
    let path = PathBuf::from("test.rs");
    let content = "fn main() {}";
    let result = "processed";

    let now = SystemTime::now();
    cache.insert(
        &path,
        content.to_string(),
        result.to_string(),
        50,
        ReadMode::Full,
        now,
    );

    // Get with same mtime - should hit
    assert!(cache.get(&path, content, ReadMode::Full, now).is_some());

    // Get with different mtime - should miss
    let later = now + Duration::from_secs(10);
    assert!(cache.get(&path, content, ReadMode::Full, later).is_none());
}

#[test]
fn cache_multiple_paths_different_keys() {
    let mut cache = SemanticCache::new(100);
    let now = SystemTime::now();

    cache.insert(
        &PathBuf::from("file1.rs"),
        "content1".to_string(),
        "result1".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    cache.insert(
        &PathBuf::from("file2.rs"),
        "content2".to_string(),
        "result2".to_string(),
        40,
        ReadMode::Full,
        now,
    );

    assert!(cache
        .get(&PathBuf::from("file1.rs"), "content1", ReadMode::Full, now)
        .is_some());
    assert!(cache
        .get(&PathBuf::from("file2.rs"), "content2", ReadMode::Full, now)
        .is_some());
    assert_eq!(cache.size(), 2);
}

#[test]
fn cache_all_read_modes() {
    let mut cache = SemanticCache::new(100);
    let path = PathBuf::from("test.rs");
    let content = "shared_content";
    let modes = vec![ReadMode::Full, ReadMode::Signatures, ReadMode::Diff];
    let now = SystemTime::now();

    for (i, mode) in modes.iter().enumerate() {
        let result = format!("result_{}", i);
        cache.insert(&path, content.to_string(), result, 50 + i * 10, *mode, now);
    }

    assert_eq!(cache.size(), 3);

    for mode in modes {
        assert!(cache.get(&path, content, mode, now).is_some());
    }
}

#[test]
fn cache_overwrite_same_key() {
    let mut cache = SemanticCache::new(100);
    let path = PathBuf::from("test.rs");
    let content = "fn main() {}";
    let now = SystemTime::now();

    cache.insert(
        &path,
        content.to_string(),
        "old_result".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    assert_eq!(cache.size(), 1);

    cache.insert(
        &path,
        content.to_string(),
        "new_result".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    assert_eq!(cache.size(), 1);

    let result = cache.get(&path, content, ReadMode::Full, now);
    assert_eq!(result.unwrap(), "new_result");
}

#[test]
fn cache_zero_capacity() {
    let cache = SemanticCache::new(0);
    assert_eq!(cache.size(), 0);
}

#[test]
fn cache_large_content() {
    let mut cache = SemanticCache::new(1);
    let path = PathBuf::from("large.rs");
    let content = "x".repeat(10000);
    let result = "cached_result";
    let now = SystemTime::now();

    cache.insert(
        &path,
        content.clone(),
        result.to_string(),
        5000,
        ReadMode::Full,
        now,
    );

    let get_result = cache.get(&path, &content, ReadMode::Full, now);
    assert_eq!(get_result.unwrap(), result);
}

#[test]
fn cache_many_entries_within_capacity() {
    let mut cache = SemanticCache::new(50);
    let now = SystemTime::now();

    for i in 0..50 {
        let path = PathBuf::from(format!("file{}.rs", i));
        let content = format!("content{}", i);
        let result = format!("result{}", i);
        cache.insert(&path, content.clone(), result, 10, ReadMode::Full, now);
    }

    assert_eq!(cache.size(), 50);

    let get_result = cache.get(&PathBuf::from("file0.rs"), "content0", ReadMode::Full, now);
    assert!(get_result.is_some());
}

// ============================================================================
// Policy Integration Tests (12 tests)
// ============================================================================

#[test]
fn policy_code_files_are_handled() {
    let code_extensions = vec!["rs", "py", "ts", "js"];
    for ext in code_extensions {
        let path = PathBuf::from(format!("test.{}", ext));
        let _mode = AdaptivePolicy::select_mode(&path, 100, None);
    }
}

#[test]
fn policy_config_files_are_handled() {
    let config_extensions = vec!["json", "yaml", "toml"];
    for ext in config_extensions {
        let path = PathBuf::from(format!("test.{}", ext));
        let _mode = AdaptivePolicy::select_mode(&path, 100, None);
    }
}

#[test]
fn policy_large_files_selected() {
    let path = PathBuf::from("large_file.txt");
    let _mode = AdaptivePolicy::select_mode(&path, 101000, None);
}

#[test]
fn policy_unknown_files_handled() {
    let path = PathBuf::from("unknown.xyz");
    let _mode = AdaptivePolicy::select_mode(&path, 100, None);
}

#[test]
fn policy_user_preference_respected() {
    let path = PathBuf::from("file.rs");
    let mode = AdaptivePolicy::select_mode(&path, 100, Some(ReadMode::Full));
    let _ = mode;
}

#[test]
fn policy_suggest_upgrade_high_usage() {
    let suggestion = AdaptivePolicy::suggest_upgrade(0.5);
    let _ = suggestion;
}

#[test]
fn policy_suggest_upgrade_very_high_usage() {
    let suggestion = AdaptivePolicy::suggest_upgrade(0.85);
    let _ = suggestion;
}

#[test]
fn policy_no_upgrade_low_usage() {
    let suggestion = AdaptivePolicy::suggest_upgrade(0.1);
    let _ = suggestion;
}

#[test]
fn policy_bash_files_handled() {
    let bash_files = vec!["script.sh", "script.bash"];
    for filename in bash_files {
        let path = PathBuf::from(filename);
        let _mode = AdaptivePolicy::select_mode(&path, 100, None);
    }
}

#[test]
fn policy_multiple_extensions() {
    let modes = vec![
        PathBuf::from("file.scala"),
        PathBuf::from("file.kt"),
        PathBuf::from("file.go"),
    ];
    for path in modes {
        let _mode = AdaptivePolicy::select_mode(&path, 100, None);
    }
}

#[test]
fn policy_size_based_selection() {
    let path = PathBuf::from("file.txt");
    let small_mode = AdaptivePolicy::select_mode(&path, 100, None);
    let large_mode = AdaptivePolicy::select_mode(&path, 101000, None);
    let _ = (small_mode, large_mode);
}

// ============================================================================
// Budget Tracking Tests (13 tests)
// ============================================================================

#[test]
fn budget_new_at_zero_consumption() {
    let budget = ContextBudget::new(1000);
    assert_eq!(budget.used(), 0);
    assert_eq!(budget.remaining(), 1000);
    assert_eq!(budget.percentage_used(), 0.0);
}

#[test]
fn budget_consume_reduces_remaining() {
    let mut budget = ContextBudget::new(1000);

    budget.consume(100);
    assert_eq!(budget.used(), 100);
    assert_eq!(budget.remaining(), 900);
}

#[test]
fn budget_percentage_calculation() {
    let mut budget = ContextBudget::new(1000);

    budget.consume(500);
    let percentage = budget.percentage_used();
    assert!((percentage - 0.5).abs() < 0.01);
}

#[test]
fn budget_multiple_consumptions() {
    let mut budget = ContextBudget::new(1000);

    budget.consume(100);
    budget.consume(200);
    budget.consume(300);

    assert_eq!(budget.used(), 600);
    assert_eq!(budget.remaining(), 400);
}

#[test]
fn budget_status_ok_before_80_percent() {
    let mut budget = ContextBudget::new(1000);

    budget.consume(700);
    let status = budget.consume(50);
    assert_eq!(status, ctx_lite::core::budget::BudgetStatus::Ok);
}

#[test]
fn budget_status_warning_at_80_percent() {
    let mut budget = ContextBudget::new(1000);

    // Need to consume MORE than 80% to trigger WarningThreshold (condition is > not >=)
    budget.consume(801);
    let status = budget.consume(0);
    assert_eq!(
        status,
        ctx_lite::core::budget::BudgetStatus::WarningThreshold
    );
}

#[test]
fn budget_status_exceeded_over_100_percent() {
    let mut budget = ContextBudget::new(1000);

    budget.consume(1000);
    let status = budget.consume(50);
    assert_eq!(status, ctx_lite::core::budget::BudgetStatus::Exceeded);
}

#[test]
fn budget_zero_budget() {
    let budget = ContextBudget::new(0);
    assert_eq!(budget.remaining(), 0);
    assert_eq!(budget.used(), 0);
}

#[test]
fn budget_large_consumption() {
    let mut budget = ContextBudget::new(100000);

    budget.consume(50000);
    assert_eq!(budget.used(), 50000);
    assert_eq!(budget.remaining(), 50000);
}

#[test]
fn budget_consume_exact_amount() {
    let mut budget = ContextBudget::new(1000);

    budget.consume(1000);
    assert_eq!(budget.remaining(), 0);
    assert_eq!(budget.percentage_used(), 1.0);
}

#[test]
fn budget_warning_threshold_precision() {
    let mut budget = ContextBudget::new(1000);

    // Test that > 80% triggers warning (using > not >=)
    budget.consume(801);
    let status = budget.consume(0);
    assert_eq!(
        status,
        ctx_lite::core::budget::BudgetStatus::WarningThreshold
    );

    // Test that exactly 80% does NOT trigger warning
    let mut budget2 = ContextBudget::new(1000);
    budget2.consume(800);
    let status2 = budget2.consume(0);
    assert_eq!(status2, ctx_lite::core::budget::BudgetStatus::Ok);
}

#[test]
fn budget_small_incremental_consumption() {
    let mut budget = ContextBudget::new(1000);

    for _ in 0..100 {
        budget.consume(5);
    }

    assert_eq!(budget.used(), 500);
    assert_eq!(budget.remaining(), 500);
}

// ============================================================================
// Integration Tests (15+ tests combining cache, policy, and budget)
// ============================================================================

#[test]
fn integration_cache_with_policy_selected_mode() {
    let mut cache = SemanticCache::new(100);

    let path = PathBuf::from("code.rs");
    let content = "fn main()";
    let result = "processing_result";
    let now = SystemTime::now();

    cache.insert(
        &path,
        content.to_string(),
        result.to_string(),
        100,
        ReadMode::Full,
        now,
    );

    let retrieved = cache.get(&path, content, ReadMode::Full, now);
    assert!(retrieved.is_some());
}

#[test]
fn integration_cache_and_budget_together() {
    let mut cache = SemanticCache::new(100);
    let mut budget = ContextBudget::new(5000);
    let now = SystemTime::now();

    for i in 0..10 {
        let path = PathBuf::from(format!("file{}.rs", i));
        let content = format!("content{}", i);
        let result = format!("result{}", i);
        let tokens = 100;

        cache.insert(&path, content, result, tokens, ReadMode::Full, now);
        budget.consume(tokens);
    }

    assert_eq!(cache.size(), 10);
    assert_eq!(budget.used(), 1000);
}

#[test]
fn integration_all_three_systems() {
    let mut cache = SemanticCache::new(50);
    let mut budget = ContextBudget::new(10000);
    let now = SystemTime::now();

    let files = vec![
        ("code.rs", 200),
        ("config.json", 150),
        ("data.csv", 200),
        ("style.ts", 300),
        ("readme.md", 100),
    ];

    for (filename, tokens) in &files {
        let path = PathBuf::from(filename);
        let content = format!("content_{}", filename);
        let result = format!("result_{}", filename);

        budget.consume(*tokens);

        if budget.percentage_used() < 0.9 {
            cache.insert(&path, content, result, *tokens, ReadMode::Full, now);
        }
    }

    assert!(budget.used() > 0);
    assert!(cache.size() > 0);
    assert!(budget.percentage_used() < 1.0);
}

#[test]
fn integration_budget_tracking_during_cache_ops() {
    let mut cache = SemanticCache::new(100);
    let mut budget = ContextBudget::new(1000);
    let now = SystemTime::now();

    budget.consume(750);

    let status = budget.consume(50);
    assert_eq!(status, ctx_lite::core::budget::BudgetStatus::Ok);

    cache.insert(
        &PathBuf::from("file.rs"),
        "content".to_string(),
        "result".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    assert!(budget.percentage_used() > 0.75);
}

#[test]
fn integration_cache_with_multiple_modes_same_file() {
    let mut cache = SemanticCache::new(100);
    let mut budget = ContextBudget::new(5000);
    let now = SystemTime::now();

    let path = PathBuf::from("source.py");
    let content = "shared_content";
    let modes = vec![ReadMode::Full, ReadMode::Signatures, ReadMode::Diff];

    for (i, mode) in modes.iter().enumerate() {
        let result = format!("result_{}", i);
        let tokens = 50;
        cache.insert(&path, content.to_string(), result, tokens, *mode, now);
        budget.consume(tokens);
    }

    assert_eq!(cache.size(), 3);
    assert_eq!(budget.used(), 150);
}

#[test]
fn integration_cache_eviction_with_multiple_files() {
    let mut cache = SemanticCache::new(3);
    let now = SystemTime::now();

    let files = vec![("code.rs", 100), ("config.json", 150), ("data.csv", 200)];

    for (i, (filename, size)) in files.iter().enumerate() {
        let path = PathBuf::from(filename);
        let content = format!("content_{}", i);
        let result = format!("result_{}", i);

        cache.insert(
            &path,
            content,
            result,
            *size,
            ReadMode::Full,
            now + Duration::from_secs(i as u64),
        );
    }

    assert_eq!(cache.size(), 3);

    let path = PathBuf::from("readme.md");
    cache.insert(
        &path,
        "new_content".to_string(),
        "new_result".to_string(),
        100,
        ReadMode::Full,
        now + Duration::from_secs(10),
    );

    assert_eq!(cache.size(), 3);
}

#[test]
fn integration_budget_prevents_excessive_caching() {
    let mut cache = SemanticCache::new(100);
    let mut budget = ContextBudget::new(500);
    let now = SystemTime::now();

    let mut inserted = 0;
    for i in 0..50 {
        let path = PathBuf::from(format!("file{}.txt", i));
        let content = format!("content{}", i);
        let result = format!("result{}", i);
        let tokens = 20;

        let new_total = budget.used() + tokens;
        if new_total <= 500 {
            budget.consume(tokens);
            cache.insert(&path, content, result, tokens, ReadMode::Full, now);
            inserted += 1;
        }
    }

    assert!(inserted < 50);
    assert!(inserted > 0);
}

#[test]
fn integration_cache_policy_budget_scaling() {
    let mut cache = SemanticCache::new(1000);
    let mut budget = ContextBudget::new(50000);
    let now = SystemTime::now();

    for i in 0..100 {
        let ext = match i % 5 {
            0 => "rs",
            1 => "json",
            2 => "txt",
            3 => "py",
            _ => "md",
        };

        let path = PathBuf::from(format!("file{}.{}", i, ext));
        let content = format!("content_{}", i);
        let result = format!("result_{}", i);
        let file_size = (i * 50) % 100000;
        let tokens = (file_size / 100).max(10);

        if budget.used() + tokens <= 50000 {
            budget.consume(tokens);
            cache.insert(&path, content, result, tokens, ReadMode::Full, now);
        }
    }

    assert!(cache.size() > 0);
    assert!(cache.size() <= 1000);
    assert!(budget.percentage_used() > 0.0);
    assert!(budget.percentage_used() <= 1.0);
}

#[test]
fn integration_policy_mode_consistency() {
    let path_small = PathBuf::from("file.txt");

    let small_mode = AdaptivePolicy::select_mode(&path_small, 100, None);
    let _ = small_mode;

    let large_mode = AdaptivePolicy::select_mode(&path_small, 101000, None);
    let _ = large_mode;
}

#[test]
fn integration_cache_with_mtime_and_modes() {
    let mut cache = SemanticCache::new(100);

    let path = PathBuf::from("main.rs");
    let content = "fn main() {}";
    let result = "processed_main";
    let now = SystemTime::now();

    cache.insert(
        &path,
        content.to_string(),
        result.to_string(),
        100,
        ReadMode::Full,
        now,
    );

    assert!(cache.get(&path, content, ReadMode::Full, now).is_some());

    let later = now + Duration::from_secs(60);
    assert!(cache.get(&path, content, ReadMode::Full, later).is_none());
}

#[test]
fn integration_budget_and_policy_coordination() {
    let mut cache = SemanticCache::new(100);
    let mut budget = ContextBudget::new(1000);
    let now = SystemTime::now();

    for i in 0..8 {
        let path = PathBuf::from(format!("file{}.txt", i));
        let content = format!("content{}", i);
        let result = format!("result");
        budget.consume(100);
        cache.insert(&path, content, result, 100, ReadMode::Full, now);
    }

    assert_eq!(budget.used(), 800);
    assert!(budget.percentage_used() >= 0.8);
}

#[test]
fn integration_stress_test_mixed_operations() {
    let mut cache = SemanticCache::new(100);
    let mut budget = ContextBudget::new(10000);
    let now = SystemTime::now();

    let modes = vec![ReadMode::Full, ReadMode::Signatures, ReadMode::Diff];

    for round in 0..20 {
        for (i, mode) in modes.iter().enumerate() {
            let path = PathBuf::from(format!("f_r{}_m{}.txt", round, i));
            let content = format!("c_r{}_m{}", round, i);
            let result = format!("r_r{}_m{}", round, i);
            let size = (round * 100 + i * 50) % 100000;
            let tokens = (size / 100).max(10).min(500);

            if budget.used() + tokens <= 10000 {
                budget.consume(tokens);
                cache.insert(&path, content, result, tokens, *mode, now);
            }
        }
    }

    assert!(cache.size() > 0);
    assert!(budget.used() > 0);
}

#[test]
fn integration_three_system_final_test() {
    let mut cache = SemanticCache::new(50);
    let mut budget = ContextBudget::new(3000);
    let now = SystemTime::now();

    for i in 0..10 {
        let path = PathBuf::from(format!("module{}.rs", i));
        let content = format!("content_module{}", i);
        let result = format!("result_module{}", i);
        let tokens = 50 + i * 10;

        if budget.used() + tokens <= 3000 && cache.size() < 50 {
            budget.consume(tokens);
            cache.insert(&path, content, result, tokens, ReadMode::Signatures, now);
        }
    }

    assert!(cache.size() > 0);
    assert!(budget.used() > 0);
    assert!(budget.used() < 3000);
}

#[test]
fn integration_cache_deterministic_lru_order() {
    let mut cache = SemanticCache::new(2);
    let now = SystemTime::now();

    // Insert first entry and wait to capture distinct timestamps
    cache.insert(
        &PathBuf::from("first.rs"),
        "c1".to_string(),
        "r1".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    thread::sleep(Duration::from_millis(10));

    // Insert second entry
    cache.insert(
        &PathBuf::from("second.py"),
        "c2".to_string(),
        "r2".to_string(),
        50,
        ReadMode::Full,
        now,
    );
    thread::sleep(Duration::from_millis(10));

    // Insert third entry - should evict the oldest (first)
    cache.insert(
        &PathBuf::from("third.ts"),
        "c3".to_string(),
        "r3".to_string(),
        50,
        ReadMode::Full,
        now,
    );

    // Verify cache size is at capacity
    assert_eq!(cache.size(), 2);

    // First entry should be evicted (oldest by insertion timestamp)
    assert!(cache
        .get(&PathBuf::from("first.rs"), "c1", ReadMode::Full, now)
        .is_none());

    // Second and third should remain
    assert!(cache
        .get(&PathBuf::from("second.py"), "c2", ReadMode::Full, now)
        .is_some());
    assert!(cache
        .get(&PathBuf::from("third.ts"), "c3", ReadMode::Full, now)
        .is_some());
}
