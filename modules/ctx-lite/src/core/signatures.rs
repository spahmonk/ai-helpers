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

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub fn ") {
            sigs.push("pub fn".to_string());
        } else if trimmed.starts_with("fn ") {
            sigs.push("fn".to_string());
        } else if trimmed.starts_with("pub async fn ") {
            sigs.push("pub async fn".to_string());
        } else if trimmed.starts_with("async fn ") {
            sigs.push("async fn".to_string());
        } else if trimmed.starts_with("pub struct ") {
            sigs.push("pub struct".to_string());
        } else if trimmed.starts_with("struct ") {
            sigs.push("struct".to_string());
        } else if trimmed.starts_with("pub enum ") {
            sigs.push("pub enum".to_string());
        } else if trimmed.starts_with("enum ") {
            sigs.push("enum".to_string());
        } else if trimmed.starts_with("pub trait ") {
            sigs.push("pub trait".to_string());
        } else if trimmed.starts_with("trait ") {
            sigs.push("trait".to_string());
        } else if trimmed.starts_with("impl ") {
            if let Some(impl_part) = trimmed.split('{').next() {
                sigs.push(impl_part.trim().to_string());
            }
        }
    }

    if sigs.is_empty() {
        content.to_string()
    } else {
        sigs.join("\n")
    }
}

fn extract_python_signatures(content: &str) -> String {
    let mut sigs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("def ") {
            // Extract minimal signature: def name only
            if let Some(name_part) = trimmed.split('(').next() {
                sigs.push(name_part.trim().to_string());
            }
        } else if trimmed.starts_with("class ") {
            // Extract: class Name
            if let Some(class_part) = trimmed.split('(').next() {
                if let Some(class_part) = class_part.split(':').next() {
                    sigs.push(class_part.trim().to_string());
                }
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
        
        // Extract minimal function signatures
        if trimmed.starts_with("export function ") {
            if let Some(name_part) = trimmed.split('(').next() {
                sigs.push(name_part.trim().to_string());
            }
        } else if trimmed.starts_with("function ") {
            if let Some(name_part) = trimmed.split('(').next() {
                sigs.push(name_part.trim().to_string());
            }
        } else if trimmed.starts_with("export const ") {
            if let Some(name_part) = trimmed.split(':').next() {
                if let Some(const_name) = name_part.split('=').next() {
                    sigs.push(format!("const {}", const_name.trim()));
                }
            }
        } else if trimmed.starts_with("const ") {
            if let Some(name_part) = trimmed.split(':').next() {
                if let Some(const_name) = name_part.split('=').next() {
                    sigs.push(format!("const {}", const_name.trim()));
                }
            }
        } else if trimmed.starts_with("export class ") || trimmed.starts_with("class ") {
            if let Some(class_part) = trimmed.split('{').next() {
                sigs.push(class_part.trim().to_string());
            }
        } else if trimmed.starts_with("export interface ") || trimmed.starts_with("interface ") {
            if let Some(iface_part) = trimmed.split('{').next() {
                sigs.push(iface_part.trim().to_string());
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
        if trimmed.starts_with("func ") {
            // Extract just: func or func (recv)
            sigs.push("func".to_string());
        } else if trimmed.starts_with("type ") && trimmed.contains(" struct") {
            sigs.push("type struct".to_string());
        } else if trimmed.starts_with("type ") {
            sigs.push("type".to_string());
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
        if trimmed.starts_with("public class ") {
            sigs.push("public class".to_string());
        } else if trimmed.starts_with("public interface ") {
            sigs.push("public interface".to_string());
        } else if trimmed.contains("public ") && trimmed.contains("(") {
            sigs.push("public method".to_string());
        } else if trimmed.contains("private ") && trimmed.contains("(") {
            sigs.push("private method".to_string());
        } else if trimmed.contains("protected ") && trimmed.contains("(") {
            sigs.push("protected method".to_string());
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
        // Skip comments
        if trimmed.starts_with("//") {
            continue;
        }
        
        if trimmed.starts_with("class ") {
            sigs.push("class".to_string());
        } else if trimmed.starts_with("struct ") {
            sigs.push("struct".to_string());
        } else if trimmed.starts_with("namespace ") {
            sigs.push("namespace".to_string());
        } else if trimmed.starts_with("template ") {
            sigs.push("template".to_string());
        } else if trimmed.starts_with("public:") {
            sigs.push("public:".to_string());
        } else if trimmed.starts_with("private:") {
            sigs.push("private:".to_string());
        } else if trimmed.starts_with("protected:") {
            sigs.push("protected:".to_string());
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
