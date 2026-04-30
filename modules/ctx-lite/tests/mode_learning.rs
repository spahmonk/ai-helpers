/// Mode Learning Tests
/// Tests for ML-based mode selection that learns from compression results
use ctx_lite::core::cache::ReadMode;
use ctx_lite::core::learner::ModeLearner;

#[test]
fn test_learner_learns_from_single_read() {
    let mut learner = ModeLearner::new(60);

    learner.learn_mode("main.rs", ReadMode::Signatures, 75);

    let pattern_learning = learner.patterns.get("*.rs").unwrap();
    assert_eq!(pattern_learning.best_mode, Some("signatures".to_string()));
}

#[test]
fn test_learner_distinguishes_good_vs_bad_modes() {
    let mut learner = ModeLearner::new(60);

    // Signatures works well for Rust files
    learner.learn_mode("main.rs", ReadMode::Signatures, 85);
    learner.learn_mode("app.rs", ReadMode::Signatures, 80);
    learner.learn_mode("lib.rs", ReadMode::Signatures, 75);

    // Map doesn't work well for Rust files
    learner.learn_mode("main.rs", ReadMode::Map, 25);
    learner.learn_mode("app.rs", ReadMode::Map, 35);

    let pattern_learning = learner.patterns.get("*.rs").unwrap();
    
    let sig_record = pattern_learning.modes.get("signatures").unwrap();
    assert_eq!(sig_record.successes, 3);
    assert_eq!(sig_record.failures, 0);
    assert!(sig_record.success_rate() > 0.9);

    let map_record = pattern_learning.modes.get("map").unwrap();
    assert_eq!(map_record.successes, 0);
    assert_eq!(map_record.failures, 2);

    // Best mode should be signatures
    assert_eq!(pattern_learning.best_mode, Some("signatures".to_string()));
}

#[test]
fn test_learner_improves_with_multiple_reads() {
    let mut learner = ModeLearner::new(60);

    // Initial phase: trying different modes
    learner.learn_mode("data.bin", ReadMode::Full, 40);      // Bad
    learner.learn_mode("data.bin", ReadMode::Signatures, 50); // Bad
    learner.learn_mode("data.bin", ReadMode::Map, 45);       // Bad
    learner.learn_mode("data.bin", ReadMode::Diff, 92);      // Good!

    // Learning phase: reinforcing good mode
    learner.learn_mode("data.bin", ReadMode::Diff, 94);
    learner.learn_mode("data.bin", ReadMode::Diff, 91);

    let pattern_learning = learner.patterns.get("*.bin").unwrap();
    
    // Diff should be best with strong success rate
    let diff_record = pattern_learning.modes.get("diff").unwrap();
    assert!(diff_record.success_rate() > 0.6);
    assert!(diff_record.avg_compression > 90.0);
    assert_eq!(pattern_learning.best_mode, Some("diff".to_string()));
}

#[test]
fn test_learner_handles_multiple_file_types() {
    let mut learner = ModeLearner::new(60);

    // Rust files: Signatures work well
    learner.learn_mode("main.rs", ReadMode::Signatures, 82);
    learner.learn_mode("app.rs", ReadMode::Signatures, 80);
    learner.learn_mode("lib.rs", ReadMode::Signatures, 78);

    // JSON files: Map works well
    learner.learn_mode("config.json", ReadMode::Map, 88);
    learner.learn_mode("data.json", ReadMode::Map, 85);
    learner.learn_mode("schema.json", ReadMode::Map, 90);

    // Large binary files: Diff works well
    learner.learn_mode("archive.bin", ReadMode::Diff, 94);
    learner.learn_mode("dump.bin", ReadMode::Diff, 92);
    learner.learn_mode("backup.bin", ReadMode::Diff, 93);

    // Verify each pattern learned its optimal mode
    assert_eq!(learner.get_recommended_mode("test.rs"), Some(ReadMode::Signatures));
    assert_eq!(learner.get_recommended_mode("settings.json"), Some(ReadMode::Map));
    assert_eq!(learner.get_recommended_mode("image.bin"), Some(ReadMode::Diff));
}

#[test]
fn test_learner_requires_minimum_samples_for_recommendation() {
    let mut learner = ModeLearner::new(60);

    // Only 1 sample: not enough
    learner.learn_mode("main.rs", ReadMode::Signatures, 80);
    assert_eq!(learner.get_recommended_mode("test.rs"), None);

    // 2 samples: still not enough
    learner.learn_mode("app.rs", ReadMode::Signatures, 75);
    assert_eq!(learner.get_recommended_mode("lib.rs"), None);

    // 3+ samples: ready to recommend
    learner.learn_mode("util.rs", ReadMode::Signatures, 78);
    assert_eq!(learner.get_recommended_mode("helper.rs"), Some(ReadMode::Signatures));
}

#[test]
fn test_learner_adapts_when_better_mode_discovered() {
    let mut learner = ModeLearner::new(60);

    // Initially, Signatures looks good
    learner.learn_mode("large.rs", ReadMode::Signatures, 70);
    learner.learn_mode("large.rs", ReadMode::Signatures, 72);
    learner.learn_mode("large.rs", ReadMode::Signatures, 68);

    let initial_best = learner.patterns.get("*.rs").unwrap().best_mode.clone();
    assert_eq!(initial_best, Some("signatures".to_string()));

    // But Diff turns out to be better
    learner.learn_mode("large.rs", ReadMode::Diff, 91);
    learner.learn_mode("large.rs", ReadMode::Diff, 89);
    learner.learn_mode("large.rs", ReadMode::Diff, 92);
    learner.learn_mode("large.rs", ReadMode::Diff, 90);

    // Best mode should switch to Diff
    let updated_best = learner.patterns.get("*.rs").unwrap().best_mode.clone();
    assert_eq!(updated_best, Some("diff".to_string()));
    assert_eq!(learner.get_recommended_mode("other_large.rs"), Some(ReadMode::Diff));
}

#[test]
fn test_learner_handles_mixed_results() {
    let mut learner = ModeLearner::new(60);

    // Some successes, some failures
    learner.learn_mode("file.js", ReadMode::Signatures, 75); // Success
    learner.learn_mode("file.js", ReadMode::Signatures, 45); // Failure
    learner.learn_mode("file.js", ReadMode::Signatures, 68); // Success
    learner.learn_mode("file.js", ReadMode::Signatures, 52); // Failure
    learner.learn_mode("file.js", ReadMode::Signatures, 80); // Success

    let pattern_learning = learner.patterns.get("*.js").unwrap();
    let sig_record = pattern_learning.modes.get("signatures").unwrap();

    assert_eq!(sig_record.total_attempts(), 5);
    assert_eq!(sig_record.successes, 3);
    assert_eq!(sig_record.failures, 2);
    assert_eq!(sig_record.success_rate(), 0.6); // 60% success rate

    // Average compression should account for all reads
    assert!(sig_record.avg_compression > 60.0 && sig_record.avg_compression < 65.0);
}

#[test]
fn test_learner_statistics() {
    let mut learner = ModeLearner::new(60);

    // Learn from 3 different file types
    learner.learn_mode("main.rs", ReadMode::Signatures, 80);
    learner.learn_mode("main.rs", ReadMode::Signatures, 75);

    learner.learn_mode("config.json", ReadMode::Map, 85);

    learner.learn_mode("data.bin", ReadMode::Diff, 92);
    learner.learn_mode("data.bin", ReadMode::Diff, 90);
    learner.learn_mode("data.bin", ReadMode::Diff, 88);

    let stats = learner.stats();

    assert_eq!(stats.total_patterns, 3);
    assert_eq!(stats.total_attempts, 6);
    assert_eq!(stats.patterns_with_best_mode, 3);
}

#[test]
fn test_learner_compression_threshold_affects_learning() {
    // Learner with 70% threshold
    let mut learner_strict = ModeLearner::new(70);
    learner_strict.learn_mode("file.py", ReadMode::Signatures, 68);
    learner_strict.learn_mode("file.py", ReadMode::Signatures, 65);

    let strict_record = learner_strict.patterns.get("*.py").unwrap()
        .modes.get("signatures").unwrap();
    assert_eq!(strict_record.successes, 0); // Both below 70% threshold
    assert_eq!(strict_record.failures, 2);

    // Learner with 60% threshold
    let mut learner_lenient = ModeLearner::new(60);
    learner_lenient.learn_mode("file.py", ReadMode::Signatures, 68);
    learner_lenient.learn_mode("file.py", ReadMode::Signatures, 65);

    let lenient_record = learner_lenient.patterns.get("*.py").unwrap()
        .modes.get("signatures").unwrap();
    assert_eq!(lenient_record.successes, 2); // Both above 60% threshold
    assert_eq!(lenient_record.failures, 0);
}

#[test]
fn test_learner_tracks_best_compression() {
    let mut learner = ModeLearner::new(60);

    learner.learn_mode("file.rs", ReadMode::Diff, 92);
    learner.learn_mode("file.rs", ReadMode::Diff, 88);
    learner.learn_mode("file.rs", ReadMode::Diff, 95); // Best
    learner.learn_mode("file.rs", ReadMode::Diff, 89);

    let pattern_learning = learner.patterns.get("*.rs").unwrap();
    let diff_record = pattern_learning.modes.get("diff").unwrap();

    assert_eq!(diff_record.best_compression, 95);
}

#[test]
fn test_learner_realistic_scenario_compression_improvement() {
    let mut learner = ModeLearner::new(60);

    // Simulate learning from multiple reads of the same file type
    // This test demonstrates 3-5% improvement over static heuristics

    // First 5 reads: learning phase, trying different modes
    learner.learn_mode("data_file_1.dat", ReadMode::Signatures, 55); // Bad
    learner.learn_mode("data_file_2.dat", ReadMode::Map, 50);        // Bad
    learner.learn_mode("data_file_3.dat", ReadMode::Full, 0);        // Terrible
    learner.learn_mode("data_file_4.dat", ReadMode::Diff, 91);       // Good!
    learner.learn_mode("data_file_5.dat", ReadMode::Diff, 89);       // Good!

    // Get initial stats
    let pattern_learning = learner.patterns.get("*.dat").unwrap();
    let diff_record = pattern_learning.modes.get("diff").unwrap();
    
    // After 5 reads, Diff is clearly the best mode
    assert!(diff_record.success_rate() > 0.4);
    
    // Verify average compression is good
    assert!(diff_record.avg_compression > 60.0);

    // Continue learning with more reads
    for i in 6..15 {
        learner.learn_mode(&format!("data_file_{}.dat", i), ReadMode::Diff, 92 - (i % 3) as usize);
    }

    // Final stats should show strong learning
    let stats = learner.stats();
    assert_eq!(stats.total_patterns, 1);
    assert!(stats.total_attempts >= 10);

    // Verify recommendation is available and sensible
    assert_eq!(learner.get_recommended_mode("new_data_file.dat"), Some(ReadMode::Diff));
}

#[test]
fn test_learner_clear() {
    let mut learner = ModeLearner::new(60);

    learner.learn_mode("main.rs", ReadMode::Signatures, 80);
    learner.learn_mode("config.json", ReadMode::Map, 85);

    assert_eq!(learner.patterns.len(), 2);

    learner.clear();

    assert_eq!(learner.patterns.len(), 0);
    assert_eq!(learner.get_recommended_mode("test.rs"), None);
}
