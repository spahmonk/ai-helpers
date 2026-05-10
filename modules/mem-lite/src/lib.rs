pub mod app;
pub mod core;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_exposed() {
        assert!(!version().is_empty());
    }
}
