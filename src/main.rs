#[tokio::main]
async fn main() {
    rusted_tui_mc_launcher::migrate_legacy_rename();

    // Guard must stay in scope to keep the log file writer alive
    let _guard = rusted_tui_mc_launcher::tui::logging::init();
    tracing::info!("Starting rusted-tui-mc-launcher {}", env!("CARGO_PKG_VERSION"));
    if let Err(e) = color_eyre::install() {
        eprintln!("Failed to install color-eyre: {}", e);
        tracing::warn!("Failed to install color-eyre handler: {}", e);
    }

    rusted_tui_mc_launcher::cli_init().await
}
