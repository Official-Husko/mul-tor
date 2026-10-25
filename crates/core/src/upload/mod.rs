//! The pieces an uploader works with: the local file, the request context, and
//! helpers that stream file data with progress reporting.

mod chunks;
mod context;
mod cookies;
mod file;
mod random;
mod url;

pub use chunks::chunk_ranges;
pub use context::{UploadContext, UploadedFile, new_form};
pub(crate) use cookies::CookieJar;
pub use file::{LocalFile, shorten_name};
pub use random::{HEX, LOWER_ALNUM, MIXED_ALNUM, random_string};
pub use url::{https_url_within, is_web_link, url_with_segments};
