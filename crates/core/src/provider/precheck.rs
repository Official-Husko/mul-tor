//! Offline checks every upload goes through before any request is made.

use super::credentials::Credentials;
use super::info::{Auth, ProviderInfo};
use crate::error::UploadError;
use crate::upload::LocalFile;

impl ProviderInfo {
    /// Cheap, offline checks every upload goes through before any request is made.
    pub fn precheck(&self, file: &LocalFile, credentials: &Credentials) -> Result<(), UploadError> {
        if file.size == 0 {
            return Err(UploadError::EmptyFile);
        }
        if let Some(limit) = self.max_size
            && file.size > limit.bytes()
        {
            return Err(UploadError::TooLarge {
                limit: limit.to_string(),
            });
        }
        let lower = file.name.to_lowercase();
        if let Some(blocked) = self
            .blocked_extensions
            .iter()
            .find(|extension| lower.ends_with(*extension))
        {
            return Err(UploadError::Rejected(format!(
                "`{blocked}` files are not allowed"
            )));
        }
        if self.auth == Auth::Required {
            for field in self.credentials {
                if credentials.get(field.key).is_none() {
                    return Err(UploadError::MissingCredential(field.key));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::provider::CredentialField;
    use crate::size::SizeLimit;

    const KEYED: ProviderInfo =
        ProviderInfo::new("Example", "https://example.com", "https://example.com")
            .max_size(SizeLimit::mb(10))
            .requires(&[CredentialField::new("apiKey", "API key", true)])
            .blocked_extensions(&[".exe"]);

    fn file(name: &str, size: u64) -> LocalFile {
        LocalFile {
            path: PathBuf::from(name),
            name: name.to_string(),
            size,
        }
    }

    #[test]
    fn precheck_rejects_empty_oversized_blocked_and_uncredentialled() {
        let mut creds = Credentials::default();
        creds.set("apiKey", "k");

        assert!(KEYED.precheck(&file("a.zip", 1024), &creds).is_ok());
        assert!(matches!(
            KEYED.precheck(&file("a.zip", 0), &creds),
            Err(UploadError::EmptyFile)
        ));
        assert!(matches!(
            KEYED.precheck(&file("a.zip", 11 * 1024 * 1024), &creds),
            Err(UploadError::TooLarge { .. })
        ));
        assert!(matches!(
            KEYED.precheck(&file("setup.EXE", 1), &creds),
            Err(UploadError::Rejected(_))
        ));
        assert!(matches!(
            KEYED.precheck(&file("a.zip", 1), &Credentials::default()),
            Err(UploadError::MissingCredential("apiKey"))
        ));
    }
}
