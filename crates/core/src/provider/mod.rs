//! What a supported provider *is*: static metadata ([`ProviderInfo`]) plus the
//! [`Provider::upload`] implementation.

mod credentials;
mod info;
mod name;
mod precheck;
mod registry;
mod reliability;

use async_trait::async_trait;

pub use credentials::{CredentialField, Credentials};
pub use info::{Auth, ProviderInfo};
pub use name::normalize_name;
pub use registry::Registry;
pub use reliability::Reliability;

use crate::error::UploadError;
use crate::upload::{UploadContext, UploadedFile};

#[async_trait]
pub trait Provider: Send + Sync {
    fn info(&self) -> &ProviderInfo;

    /// Upload `cx.file` and return its public link.
    ///
    /// The engine has already run [`ProviderInfo::precheck`] and takes care of
    /// cancellation (the future is simply dropped), so implementations only
    /// deal with the protocol.
    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError>;
}
