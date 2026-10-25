//! Panics end up in the log, with where they happened.

use std::panic::PanicHookInfo;

/// Log every panic, then run the hook that was installed before (which
/// prints the usual message).
pub(super) fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(
            location = %location(info),
            reason = %message(info),
            "the program panicked"
        );
        previous(info);
    }));
}

fn location(info: &PanicHookInfo<'_>) -> String {
    info.location()
        .map(|at| format!("{}:{}", at.file(), at.line()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn message(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    payload
        .downcast_ref::<&str>()
        .map(|text| text.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(no message)".to_string())
}
