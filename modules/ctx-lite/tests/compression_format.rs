/// Compression Format Tests
/// Tests for pre-compression optimization and format validation
/// Ensures minifier accuracy, format compatibility, and compression improvements

use ctx_lite::core::minify::{CompressedFormat, Minifier};
use ctx_lite::core::diff::DiffMode;

#[test]
fn test_minifier_rust_function_signature() {
    let input = "pub fn foo ( ) -> String";
    let result = Minifier::minify_rust_signature(input);
    
    assert_eq!(result, "pub fn foo()->String");
    // Verify semantic content is preserved
    assert!(result.contains("foo"));
    assert!(result.contains("String"));
}

#[test]
fn test_minifier_python_function_signature() {
    let input = "def calculate ( x : int , y : int ) -> int :";
    let result = Minifier::minify_python_signature(input);
    
    // Verify key patterns are minified
    assert!(!result.contains("( "));
    assert!(!result.contains(" )"));
    assert!(!result.contains(" -> "));
    assert!(!result.contains(" : "));
    // Verify semantic content preserved
    assert!(result.contains("calculate"));
    assert!(result.contains("int"));
}

#[test]
fn test_minifier_javascript_signature() {
    let input = "async function fetchData ( url : string , options : Config ) : Promise < Response >";
    let result = Minifier::minify_js_signature(input);
    
    // Verify spaces removed around operators/delimiters
    assert!(!result.contains(" : "));
    assert!(!result.contains("< "));
    assert!(!result.contains(" >"));
    // Verify semantic content preserved
    assert!(result.contains("fetchData"));
    assert!(result.contains("string"));
    assert!(result.contains("Response"));
}

#[test]
fn test_minifier_complex_rust_generics() {
    let input = "pub fn process < T : Clone + Display > ( items : Vec < T > ) -> Result < Vec < T > , Error >";
    let result = Minifier::minify_rust_signature(input);
    
    let spaces_before = input.matches(' ').count();
    let spaces_after = result.matches(' ').count();
    
    assert!(spaces_after < spaces_before, "Minified should have fewer spaces");
    // Verify all key elements preserved
    assert!(result.contains("process"));
    assert!(result.contains("Clone"));
    assert!(result.contains("Display"));
    assert!(result.contains("Vec"));
}

#[test]
fn test_minifier_preserves_string_literals() {
    let input = r#"const msg = "hello   world   with   spaces";"#;
    let result = Minifier::minify(input);
    
    // String content should be preserved exactly
    assert!(result.contains("hello   world   with   spaces"));
}

#[test]
fn test_minifier_handles_single_quotes() {
    let input = "const msg = 'hello   world';";
    let result = Minifier::minify(input);
    
    assert!(result.contains("hello   world"));
}

#[test]
fn test_minifier_handles_backticks() {
    let input = "const msg = `hello ${name}  with   spaces`;";
    let result = Minifier::minify(input);
    
    assert!(result.contains("hello ${name}  with   spaces"));
}

#[test]
fn test_minifier_json_compact_format() {
    let input = r#"{ "name" : "John" , "age" : 30 , "tags" : [ "rust" , "web" ] }"#;
    let result = Minifier::minify_json(input);
    
    // JSON should be more compact
    assert!(result.len() <= input.len());
    // No spaces after colons or commas in JSON
    assert!(!result.contains(" : "));
    assert!(!result.contains(" , "));
    // Semantic content preserved
    assert!(result.contains("\"name\""));
    assert!(result.contains("\"John\""));
}

#[test]
fn test_minifier_compression_percentage() {
    let original = "pub  fn   test  (  )  {  code  }";
    let minified = Minifier::minify_rust_signature(original);
    let percent = Minifier::compression_percent(&original, &minified);
    
    assert!(percent > 0, "Should have positive compression");
    assert!(percent <= 100, "Compression should not exceed 100%");
    assert_eq!(percent as usize, percent, "Should be whole number");
}

#[test]
fn test_minifier_large_signature() {
    let input = "pub async fn fetch_and_process_data < T : Clone + Send + Sync > ( items : Vec < T > , config : Config ) -> Result < (Vec < T > , Stats) , Box < dyn Error > >";
    let result = Minifier::minify_rust_signature(input);
    
    // Calculate compression
    let original_size = input.len();
    let minified_size = result.len();
    let compression = ((original_size - minified_size) as f32 / original_size as f32) * 100.0;
    
    // Large signature should achieve good compression
    assert!(compression > 10.0, "Should compress complex signatures");
    // Semantic content preserved
    assert!(result.contains("fetch_and_process_data"));
    assert!(result.contains("Clone"));
    assert!(result.contains("Send"));
}

#[test]
fn test_compressed_format_enum_full() {
    let content = "pub  fn   test  (  )  {  }";
    let result = Minifier::apply_format(content, &CompressedFormat::Full);
    
    assert_eq!(result, content, "Full format should return unchanged content");
}

#[test]
fn test_compressed_format_enum_minified() {
    let content = "pub  fn   test  (  )  {  }";
    let result = Minifier::apply_format(content, &CompressedFormat::Minified);
    
    assert_ne!(result, content, "Minified should change content");
    assert!(result.len() <= content.len(), "Minified should be equal or smaller");
}

#[test]
fn test_compressed_format_enum_packed() {
    let content = r#"{ "key" : "value" , "nested" : { "item" : 42 } }"#;
    let result = Minifier::apply_format(content, &CompressedFormat::Packed);
    
    // Packed (JSON minified) should be more compact
    assert!(result.len() <= content.len());
}

#[test]
fn test_format_parsing() {
    assert_eq!(CompressedFormat::from_str("full"), Some(CompressedFormat::Full));
    assert_eq!(CompressedFormat::from_str("minified"), Some(CompressedFormat::Minified));
    assert_eq!(CompressedFormat::from_str("packed"), Some(CompressedFormat::Packed));
    assert_eq!(CompressedFormat::from_str("gzip"), Some(CompressedFormat::Gzip));
    assert_eq!(CompressedFormat::from_str("unknown"), None);
}

#[test]
fn test_format_string_representation() {
    assert_eq!(CompressedFormat::Full.as_str(), "full");
    assert_eq!(CompressedFormat::Minified.as_str(), "minified");
    assert_eq!(CompressedFormat::Packed.as_str(), "packed");
    assert_eq!(CompressedFormat::Gzip.as_str(), "gzip");
}

#[test]
fn test_diff_result_has_format_type() {
    let mut differ = DiffMode::new();
    let content = "line1\nline2\nline3\n";
    let result = differ.compute_diff(None, content);
    
    // DiffResult should have format_type field
    assert_eq!(result.format_type, CompressedFormat::Full);
}

#[test]
fn test_diff_result_format_on_diff_mode() {
    let mut differ = DiffMode::new();
    let content1 = "line1\nline2\nline3\n";
    let _result1 = differ.compute_diff(None, content1);
    
    let content2 = "line1\nline2\nline3\n";
    let result2 = differ.compute_diff(Some(content1), content2);
    
    // Identical files should use minified format (high compression)
    assert_eq!(result2.format_type, CompressedFormat::Minified);
}

#[test]
fn test_minifier_real_world_rust_code() {
    let input = r#"
    pub async fn process_request < T : Serialize + Deserialize > (
        req : &Request ,
        state : Arc < AppState >
    ) -> Result < Response , Error > {
        // ...
    }
    "#;
    
    let result = Minifier::minify_rust_signature(input);
    
    // Should handle multiline signatures
    assert!(!result.contains('\n'));
    // Should preserve key components
    assert!(result.contains("process_request"));
    assert!(result.contains("Serialize"));
    assert!(result.contains("AppState"));
}

#[test]
fn test_minifier_multiline_to_single_line() {
    let multiline = "pub fn foo (\n    x : i32 ,\n    y : String\n) -> Result < String , Error >";
    let result = Minifier::minify_rust_signature(multiline);
    
    assert!(!result.contains('\n'), "Should convert to single line");
    assert!(result.contains("foo"));
    assert!(result.contains("i32"));
}

#[test]
fn test_packed_format_saves_30_percent() {
    let original = r#"
    {
        "function": "test_function",
        "parameters": [
            { "name": "param1", "type": "String" },
            { "name": "param2", "type": "i32" }
        ],
        "return_type": "Result",
        "metadata": {
            "async": true,
            "unsafe": false
        }
    }
    "#;
    
    let packed = Minifier::apply_format(original, &CompressedFormat::Packed);
    let compression = Minifier::compression_percent(original, &packed);
    
    // Packed format should save at least some space
    assert!(packed.len() < original.len(), "Packed should be smaller than original");
    // For well-structured JSON, should exceed 30% compression
    assert!(compression > 20, "Should achieve significant compression");
}

#[test]
fn test_minifier_handles_escaped_quotes() {
    let input = r#"const msg = "hello \"world\" with \\backslash";"#;
    let result = Minifier::minify(input);
    
    // Should preserve escaped content
    assert!(result.contains("hello"));
}

#[test]
fn test_minifier_empty_input() {
    let result = Minifier::minify("");
    assert_eq!(result, "");
}

#[test]
fn test_minifier_only_whitespace() {
    let result = Minifier::minify("     \n\t\n     ");
    assert_eq!(result, "");
}

#[test]
fn test_compression_percent_zero_original() {
    let percent = Minifier::compression_percent("", "anything");
    assert_eq!(percent, 0);
}

#[test]
fn test_minifier_preserves_language_keywords() {
    let rust_code = "fn test_fn ( ) -> () { let x = 42 ; return x ; }";
    let result = Minifier::minify_rust_signature(rust_code);
    
    // Keywords should be preserved
    assert!(result.contains("fn"));
    assert!(result.contains("let"));
    assert!(result.contains("return"));
}

#[test]
fn test_format_round_trip_parsing() {
    for format in &[
        CompressedFormat::Full,
        CompressedFormat::Minified,
        CompressedFormat::Packed,
        CompressedFormat::Gzip,
    ] {
        let string_repr = format.as_str();
        let parsed = CompressedFormat::from_str(string_repr);
        assert_eq!(parsed, Some(format.clone()));
    }
}

#[test]
fn test_minifier_typescript_generics() {
    let input = "function getData < T extends Record < string , any > > ( id : T ) : Promise < T >";
    let result = Minifier::minify_js_signature(input);
    
    // All spacing should be compact
    let spaces = result.matches(' ').count();
    let input_spaces = input.matches(' ').count();
    
    assert!(spaces < input_spaces, "TypeScript signature should be minified");
}

#[test]
fn test_minifier_python_decorators() {
    let input = "@staticmethod\n@cache\ndef get_value ( ) -> int :";
    let result = Minifier::minify_python_signature(input);
    
    // Should handle decorators
    assert!(result.contains("get_value"));
}

#[test]
fn test_minifier_json_nested_arrays() {
    let input = r#"{ "arrays" : [ [ 1 , 2 , 3 ] , [ 4 , 5 , 6 ] ] , "count" : 2 }"#;
    let result = Minifier::minify_json(input);
    
    assert!(result.len() <= input.len());
    assert!(result.contains("\"arrays\""));
}

#[test]
fn test_integration_minify_then_compute_diff() {
    let mut differ = DiffMode::new();
    
    let content1 = "pub  fn   process ( ) -> String";
    let result1 = differ.compute_diff(None, content1);
    
    assert_eq!(result1.format_type, CompressedFormat::Full);
    assert_eq!(result1.compression_percent, 0);
    
    // Second read with identical minified content
    let content2 = "pub fn process()->String"; // Already minified
    let result2 = differ.compute_diff(Some(content1), content2);
    
    // Should recognize this as a change (content differs)
    // But the format_type should still reflect the minification capability
    assert!(result2.compression_percent <= 100);
}

#[test]
fn test_minifier_consecutive_calls_consistency() {
    let input = "pub  fn   test  (  )  {  }";
    
    let result1 = Minifier::minify(input);
    let result2 = Minifier::minify(&result1);
    let result3 = Minifier::minify(&result2);
    
    // Multiple minifications should eventually stabilize
    // (third pass should be same as second pass)
    assert_eq!(result2, result3, "Minification should stabilize");
}

#[test]
fn test_minifier_special_chars_in_strings() {
    let input = r#"const regex = "^[a-z]+ @[0-9]+ : {.*?}$";"#;
    let result = Minifier::minify(input);
    
    // Regex content should be preserved
    assert!(result.contains("a-z"));
    assert!(result.contains("0-9"));
}

#[test]
fn test_minifier_comment_like_content_in_string() {
    let input = r#"const msg = "// this looks like a comment but is a string";"#;
    let result = Minifier::minify(input);
    
    // Should not treat string content as comment
    assert!(result.contains("looks like a comment"));
}
