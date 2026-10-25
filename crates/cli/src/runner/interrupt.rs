//! Ctrl-C while an upload runs.

use console::style;
use tokio_util::sync::CancellationToken;

/// First Ctrl-C cancels the run cleanly; a second one exits immediately.
pub async fn watch_for_ctrl_c(cancel: CancellationToken) {
    if tokio::signal::ctrl_c().await.is_err() {
        return;
    }
    eprintln!(
        "\n{}",
        style("Cancelling... (press Ctrl-C again to quit now)").yellow()
    );
    cancel.cancel();
    if tokio::signal::ctrl_c().await.is_ok() {
        std::process::exit(130);
    }
}
