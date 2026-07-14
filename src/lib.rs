// crate root. main.rs is a thin wrapper that imports the two entry points
// re-exported below; everything else stays crate-private. integration tests
// in tests/ that need to reach in deeper can use `RTML::auth`, `RTML::net`,
// etc. directly; cli + migrate stay private because they have nothing
// general to expose.

pub mod auth;
mod cli;
pub mod config;
pub mod instance;
pub mod launch_profile;
pub mod net;
pub mod online;
pub mod tui;

pub use cli::init as cli_init;
pub use config::migrate::run_legacy_rename as migrate_legacy_rename;

/// 许可证全文，编译时嵌入。
pub const LICENSE_TEXT: &str = include_str!("../LICENSE");
