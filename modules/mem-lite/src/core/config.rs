use std::path::PathBuf;

pub fn default_mem_lite_home() -> PathBuf {
    if let Ok(home) = std::env::var("MEM_LITE_HOME") {
        return PathBuf::from(home);
    }

    if let Ok(home) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(home).join("mem-lite");
    }

    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        return PathBuf::from(home).join(".mem-lite");
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".mem-lite")
}
