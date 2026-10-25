//! What a command prints: progress bars for people, JSON lines for scripts,
//! and the closing summary.

mod json;
mod progress;
mod summary;
#[cfg(test)]
mod testing;

pub use progress::Renderer;
