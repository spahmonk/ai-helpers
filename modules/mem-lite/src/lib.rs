pub mod app;
pub mod core;

pub use core::config::default_mem_lite_home;
pub use core::project::{ProjectError, ProjectScope};
pub use core::store::{
    MemoryLevel, MemorySource, MemoryStats, MemoryStore, RecentMemory, RememberInput, StoreError,
};

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
