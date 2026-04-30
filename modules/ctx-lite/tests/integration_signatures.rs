/// Integration tests for signature extraction with compression ratio verification
/// Tests verify that extracted signatures compress code to <10% of original size
use std::fs;
use std::path::PathBuf;

fn read_fixture(filename: &str) -> String {
    let path = PathBuf::from("tests/fixtures/signatures").join(filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {:?}", path))
}

fn calculate_ratio(extracted: &str, original: &str) -> f64 {
    (extracted.len() as f64 / original.len() as f64) * 100.0
}

fn get_fixture_path(filename: &str) -> PathBuf {
    PathBuf::from("tests/fixtures/signatures").join(filename)
}

// ============================================================================
// RUST SIGNATURE EXTRACTION TESTS
// ============================================================================

#[test]
fn test_rust_signatures_extraction() {
    let fixture = read_fixture("sample.rs");
    let path = get_fixture_path("sample.rs");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);

    let ratio = calculate_ratio(&result, &fixture);
    println!("Rust compression ratio: {:.2}%", ratio);
    assert!(
        ratio < 10.0,
        "Rust compression ratio: {:.2}% (expected <10%)",
        ratio
    );

    // Verify content preservation
    assert!(
        result.contains("pub fn ") || result.contains("fn "),
        "Missing fn signatures"
    );
    assert!(
        result.contains("pub struct ") || result.contains("struct "),
        "Missing struct signatures"
    );
    assert!(
        result.contains("impl "),
        "Missing impl signatures"
    );

    // Verify implementation code is stripped
    assert!(
        !result.contains("println!"),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("unwrap()"),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("let "),
        "Variable assignments leaked into signatures"
    );

    println!("✓ Rust signatures extraction passed");
}

#[test]
fn test_rust_signatures_not_empty() {
    let fixture = read_fixture("sample.rs");
    let path = get_fixture_path("sample.rs");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
    assert!(!result.is_empty(), "Rust signatures should not be empty");
    assert!(result.len() < fixture.len(), "Rust signatures should be smaller than original");
}

// ============================================================================
// PYTHON SIGNATURE EXTRACTION TESTS
// ============================================================================

#[test]
fn test_python_signatures_extraction() {
    let fixture = read_fixture("sample.py");
    let path = get_fixture_path("sample.py");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);

    let ratio = calculate_ratio(&result, &fixture);
    println!("Python compression ratio: {:.2}%", ratio);
    assert!(
        ratio < 10.0,
        "Python compression ratio: {:.2}% (expected <10%)",
        ratio
    );

    // Verify content preservation
    assert!(
        result.contains("def "),
        "Missing def signatures"
    );
    assert!(
        result.contains("class "),
        "Missing class signatures"
    );

    // Verify implementation code is stripped
    assert!(
        !result.contains("print("),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("return "),
        "Return statements leaked into signatures"
    );
    assert!(
        !result.contains("    "),
        "Body indentation leaked into signatures"
    );

    println!("✓ Python signatures extraction passed");
}

#[test]
fn test_python_signatures_not_empty() {
    let fixture = read_fixture("sample.py");
    let path = get_fixture_path("sample.py");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
    assert!(!result.is_empty(), "Python signatures should not be empty");
    assert!(result.len() < fixture.len(), "Python signatures should be smaller than original");
}

// ============================================================================
// TYPESCRIPT SIGNATURE EXTRACTION TESTS
// ============================================================================

#[test]
fn test_typescript_signatures_extraction() {
    let fixture = read_fixture("sample.ts");
    let path = get_fixture_path("sample.ts");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);

    let ratio = calculate_ratio(&result, &fixture);
    println!("TypeScript compression ratio: {:.2}%", ratio);
    assert!(
        ratio < 10.0,
        "TypeScript compression ratio: {:.2}% (expected <10%)",
        ratio
    );

    // Verify content preservation
    assert!(
        result.contains("function ") || result.contains("export "),
        "Missing function/export signatures"
    );
    assert!(
        result.contains("class "),
        "Missing class signatures"
    );
    assert!(
        result.contains("interface "),
        "Missing interface signatures"
    );

    // Verify implementation code is stripped
    assert!(
        !result.contains("console.log"),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("async for"),
        "Loop statements leaked into signatures"
    );
    assert!(
        !result.contains("return "),
        "Return statements leaked into signatures"
    );

    println!("✓ TypeScript signatures extraction passed");
}

#[test]
fn test_typescript_signatures_not_empty() {
    let fixture = read_fixture("sample.ts");
    let path = get_fixture_path("sample.ts");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
    assert!(!result.is_empty(), "TypeScript signatures should not be empty");
    assert!(result.len() < fixture.len(), "TypeScript signatures should be smaller than original");
}

// ============================================================================
// GO SIGNATURE EXTRACTION TESTS
// ============================================================================

#[test]
fn test_go_signatures_extraction() {
    let fixture = read_fixture("sample.go");
    let path = get_fixture_path("sample.go");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);

    let ratio = calculate_ratio(&result, &fixture);
    println!("Go compression ratio: {:.2}%", ratio);
    assert!(
        ratio < 10.0,
        "Go compression ratio: {:.2}% (expected <10%)",
        ratio
    );

    // Verify content preservation
    assert!(
        result.contains("func "),
        "Missing func signatures"
    );
    assert!(
        result.contains("type "),
        "Missing type signatures"
    );

    // Verify implementation code is stripped
    assert!(
        !result.contains("fmt."),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("if "),
        "Control flow leaked into signatures"
    );
    assert!(
        !result.contains("for "),
        "Loop statements leaked into signatures"
    );

    println!("✓ Go signatures extraction passed");
}

#[test]
fn test_go_signatures_not_empty() {
    let fixture = read_fixture("sample.go");
    let path = get_fixture_path("sample.go");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
    assert!(!result.is_empty(), "Go signatures should not be empty");
    assert!(result.len() < fixture.len(), "Go signatures should be smaller than original");
}

// ============================================================================
// JAVA SIGNATURE EXTRACTION TESTS
// ============================================================================

#[test]
fn test_java_signatures_extraction() {
    let fixture = read_fixture("sample.java");
    let path = get_fixture_path("sample.java");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);

    let ratio = calculate_ratio(&result, &fixture);
    println!("Java compression ratio: {:.2}%", ratio);
    assert!(
        ratio < 10.0,
        "Java compression ratio: {:.2}% (expected <10%)",
        ratio
    );

    // Verify content preservation
    assert!(
        result.contains("public "),
        "Missing public modifiers"
    );
    assert!(
        result.contains("class ") || result.contains("interface "),
        "Missing class/interface signatures"
    );

    // Verify implementation code is stripped
    assert!(
        !result.contains("System.out"),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("throw "),
        "Exception code leaked into signatures"
    );
    assert!(
        !result.contains("try "),
        "Try blocks leaked into signatures"
    );

    println!("✓ Java signatures extraction passed");
}

#[test]
fn test_java_signatures_not_empty() {
    let fixture = read_fixture("sample.java");
    let path = get_fixture_path("sample.java");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
    assert!(!result.is_empty(), "Java signatures should not be empty");
    assert!(result.len() < fixture.len(), "Java signatures should be smaller than original");
}

// ============================================================================
// C++ SIGNATURE EXTRACTION TESTS
// ============================================================================

#[test]
fn test_cpp_signatures_extraction() {
    let fixture = read_fixture("sample.cpp");
    let path = get_fixture_path("sample.cpp");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);

    let ratio = calculate_ratio(&result, &fixture);
    println!("C++ compression ratio: {:.2}%", ratio);
    assert!(
        ratio < 10.0,
        "C++ compression ratio: {:.2}% (expected <10%)",
        ratio
    );

    // Verify content preservation
    assert!(
        result.contains("class ") || result.contains("struct "),
        "Missing class/struct signatures"
    );
    assert!(
        result.contains("(") && result.contains(")"),
        "Missing function signatures"
    );

    // Verify implementation code is stripped
    assert!(
        !result.contains("std::cout"),
        "Implementation code leaked into signatures"
    );
    assert!(
        !result.contains("throw "),
        "Exception code leaked into signatures"
    );
    assert!(
        !result.contains("for "),
        "Loop statements leaked into signatures"
    );

    println!("✓ C++ signatures extraction passed");
}

#[test]
fn test_cpp_signatures_not_empty() {
    let fixture = read_fixture("sample.cpp");
    let path = get_fixture_path("sample.cpp");
    let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
    assert!(!result.is_empty(), "C++ signatures should not be empty");
    assert!(result.len() < fixture.len(), "C++ signatures should be smaller than original");
}

// ============================================================================
// SUMMARY TESTS
// ============================================================================

#[test]
fn test_all_languages_compression_summary() {
    let fixtures = vec![
        ("sample.rs", "Rust"),
        ("sample.py", "Python"),
        ("sample.ts", "TypeScript"),
        ("sample.go", "Go"),
        ("sample.java", "Java"),
        ("sample.cpp", "C++"),
    ];

    println!("\n=== Signature Extraction Compression Summary ===\n");
    println!("{:<12} {:<15} {:<15} {:<10}", "Language", "Original (bytes)", "Extracted (bytes)", "Ratio");
    println!("{}", "-".repeat(52));

    let mut all_ratios_valid = true;

    for (filename, lang_name) in fixtures.iter() {
        let fixture = read_fixture(filename);
        let path = get_fixture_path(filename);
        let result = ctx_lite::core::signatures::extract_signatures(&fixture, &path);
        let ratio = calculate_ratio(&result, &fixture);

        println!(
            "{:<12} {:<15} {:<15} {:<9.2}%",
            lang_name, fixture.len(), result.len(), ratio
        );

        if ratio >= 10.0 {
            all_ratios_valid = false;
        }
    }

    println!("{}", "-".repeat(52));
    println!("\n✓ All compression ratios are within acceptable range (<10%)\n");

    assert!(
        all_ratios_valid,
        "Some languages exceed 10% compression ratio"
    );
}
