/// The crate version, single-sourced from `Cargo.toml`. Bump the version there,
/// tag the commit `vX.Y.Z`, and push — the release workflow publishes to
/// crates.io.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
