//! Records other crates write through the `log` facade (the GPU stack does)
//! show up in the buffer with their own source, not as bridge noise.
//!
//! The logger is process-wide, so this file holds a single test.

use mul_tor_core::logging::{self, LogBuffer, LogConfig};
use tracing::Level;

#[test]
fn log_records_keep_their_target_and_lose_the_bridge_fields() {
    let buffer = LogBuffer::new(50);
    let guard = logging::init(LogConfig::new().buffer(buffer.clone()).level(Level::INFO));
    assert!(guard.problems().is_empty(), "{:?}", guard.problems());

    log::warn!(target: "egui_wgpu", "Software rasterizer detected");
    tracing::warn!(provider = "Oshi", "native tracing record");

    let records = buffer.snapshot();
    let bridged = records
        .iter()
        .find(|r| r.message == "Software rasterizer detected")
        .expect("the log record reached the buffer");
    assert_eq!(bridged.target, "egui_wgpu");
    assert_eq!(bridged.level, Level::WARN);
    assert!(
        bridged.fields.is_empty(),
        "no log.* fields expected: {:?}",
        bridged.fields
    );
    assert!(
        bridged.location.contains("log_bridge.rs:"),
        "{}",
        bridged.location
    );

    let native = records
        .iter()
        .find(|r| r.message == "native tracing record")
        .unwrap();
    assert_eq!(
        native.fields,
        [("provider".to_string(), "Oshi".to_string())]
    );
}
