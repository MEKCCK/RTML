mod render;
mod state;

pub use render::{popup_rect, render};
pub use state::{DownloadStep, DownloadState, InstallParams, handle_key, take_result};
