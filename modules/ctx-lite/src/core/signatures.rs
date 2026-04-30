/// Signature extraction for code files (95%+ token savings)
use std::path::Path;

/// Extract function/class/method signatures from code
pub fn extract_signatures(content: &str, path: &Path) -> String {
    let file_ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match file_ext {
        "rs" => extract_rust_signatures(content),
        "py" => extract_python_signatures(content),
        "ts" | "tsx" => extract_typescript_signatures(content),
        "js" | "jsx" => extract_javascript_signatures(content),
        "go" => extract_go_signatures(content),
        "java" => extract_java_signatures(content),
        "cpp" | "cc" | "h" | "hpp" => extract_cpp_signatures(content),
        "c" => extract_c_signatures(content),
        _ => format!("(unsupported language: {})", file_ext),
    }
}

fn extract_rust_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    // Function definitions
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("async fn ")
        {
            if let Some(sig) = extract_rust_fn_sig(trimmed) {
                sigs.push(sig);
            }
        } else if trimmed.starts_with("pub struct ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("trait ")
        {
            sigs.push(trimmed.split('{').next().unwrap_or(trimmed).trim().to_string());
        } else if trimmed.starts_with("impl ") {
            if let Some(impl_part) = trimmed.split('{').next() {
                sigs.push(impl_part.to_string());
            }
        }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_rust_fn_sig(line: &str) -> Option<String> {
    // Extract function signature up to opening brace
    line.split('{').next().map(|s| s.trim().to_string())
}

fn extract_python_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
            if let Some(sig) = trimmed.split(':').next() {
                sigs.push(sig.trim().to_string());
            }
        }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_typescript_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("export function ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("export const ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("export class ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("export interface ")
            || trimmed.starts_with("interface ")
        {
            if let Some(sig) = trimmed.split('{').next() {
                sigs.push(sig.trim().to_string());
            }
        }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_javascript_signatures(content: &str) -> String {
    extract_typescript_signatures(content)
}

fn extract_go_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("func ") || trimmed.starts_with("type ") {
            if let Some(sig) = trimmed.split('{').next() {
                sigs.push(sig.trim().to_string());
            }
        }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_java_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("public ")
            || trimmed.contains("private ")
            || trimmed.contains("class ")
            || trimmed.contains("interface ")
        {
            if let Some(sig) = trimmed.split('{').next() {
                sigs.push(sig.trim().to_string());
            }
        }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_cpp_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // Look for function declarations and class definitions
        if ((trimmed.contains('(') && trimmed.contains(')'))
            || trimmed.starts_with("class ")
            || trimmed.starts_with("struct "))
            && !trimmed.starts_with("//") {
                if let Some(sig) = trimmed.split('{').next() {
                    sigs.push(sig.trim().to_string());
                }
            }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_c_signatures(content: &str) -> String {
    extract_cpp_signatures(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_rust_signatures() {
        let code = r#"
pub fn main() {
    println!("hello");
}

fn helper(x: i32) -> i32 {
    x + 1
}

pub struct Config {
    name: String,
}
"#;
        let sigs = extract_rust_signatures(code);
        assert!(sigs.contains("pub fn main()"));
        assert!(sigs.contains("fn helper(x: i32) -> i32"));
        assert!(sigs.contains("pub struct Config"));
    }

    #[test]
    fn test_extract_python_signatures() {
        let code = r#"
def hello(name):
    print(name)

class MyClass:
    def method(self):
        pass
"#;
        let sigs = extract_python_signatures(code);
        assert!(sigs.contains("def hello(name)"));
        assert!(sigs.contains("class MyClass"));
    }
}
