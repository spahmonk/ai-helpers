use std::fs;
use std::path::PathBuf;

fn main() {
    let fixture_dir = PathBuf::from("modules/ctx-lite/tests/fixtures/signatures");
    
    for filename in &["sample.rs", "sample.py", "sample.ts"] {
        let path = fixture_dir.join(filename);
        let content = fs::read_to_string(&path).unwrap();
        let result = ctx_lite::core::signatures::extract_signatures(&content, &path);
        
        println!("\n=== {} ===", filename);
        println!("Original size: {} bytes", content.len());
        println!("Extracted size: {} bytes", result.len());
        println!("Ratio: {:.2}%\n", (result.len() as f64 / content.len() as f64) * 100.0);
        println!("First 500 chars of extracted:\n{}", &result[..std::cmp::min(500, result.len())]);
    }
}
