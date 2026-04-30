pub mod app;
pub mod core;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
