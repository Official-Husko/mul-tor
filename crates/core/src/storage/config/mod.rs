//! User settings (`config.toml`) and import of the Python version's `config.json`.

mod accounts;
mod legacy;
mod model;
mod store;

pub use model::{CONFIG_VERSION, Config, PresetSettings};
pub use store::{ConfigOrigin, ConfigStore};
