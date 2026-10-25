//! Recording finished uploads, and the `{placeholder}` link templates presets use.

mod entry;
mod links;
mod log;
mod template;

pub use entry::LinkEntry;
pub use links::{FinishedLink, render_links};
pub use log::LinkLog;
pub use template::{format_template, template_fields};
