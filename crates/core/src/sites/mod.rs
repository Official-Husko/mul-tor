//! The built-in sites.
//!
//! Adding a site: see `docs/ADDING_SITES.md`.

use std::sync::Arc;

use crate::site::Site;

mod catbox;

/// Every built-in site, in the order they are listed to the user.
pub fn all() -> Vec<Arc<dyn Site>> {
    vec![
        Arc::new(catbox::Catbox),
        Arc::new(catbox::LitterBox),
    ]
}
