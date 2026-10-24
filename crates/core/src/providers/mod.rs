//! The built-in providers.
//!
//! Adding a provider: see `docs/ADDING_PROVIDERS.md`.
//!
//! Providers that share a protocol are declared as data in a family module
//! (`xfs`, `jqfu`); the rest get a file of their own. The Dropzone helper is
//! used by several of those individual files.

use std::sync::Arc;

use crate::provider::Provider;

mod dropzone;
mod jqfu;
mod xfs;

mod anontransfer;
mod anonymfile;
mod bowfile;
mod buzzheavier;
mod catbox;
mod delafil;
mod doodrive;
mod downloadgg;
mod easyupload;
mod fastupload;
mod filebin;
mod fileio;
mod filesdpua;
mod filesfm;
mod filesi;
mod fileupload;
mod gofile;
mod keep;
mod krakenfiles;
mod mixdrop;
mod onefichier;
mod oshi;
mod pixeldrain;
mod rapidgator;
mod tempsend;
mod tmpfiles;
mod transfersh;
mod ufile;

pub use anontransfer::AnonTransfer;
pub use anonymfile::Anonymfile;
pub use bowfile::BowFile;
pub use buzzheavier::Buzzheavier;
pub use catbox::{Catbox, LitterBox};
pub use delafil::Delafil;
pub use doodrive::DooDrive;
pub use downloadgg::DownloadGg;
pub use easyupload::EasyUpload;
pub use fastupload::Fastupload;
pub use filebin::FileBin;
pub use fileio::FileIo;
pub use filesdpua::FilesDpUa;
pub use filesfm::FilesFm;
pub use filesi::FileSi;
pub use fileupload::FileUpload;
pub use gofile::Gofile;
pub use jqfu::JqfuProvider;
pub use keep::Keep;
pub use krakenfiles::Krakenfiles;
pub use mixdrop::Mixdrop;
pub use onefichier::OneFichier;
pub use oshi::Oshi;
pub use pixeldrain::Pixeldrain;
pub use rapidgator::Rapidgator;
pub use tempsend::TempSend;
pub use tmpfiles::TmpFiles;
pub use transfersh::TransferSh;
pub use ufile::UFile;
pub use xfs::XfsProvider;

/// Every built-in provider, sorted by name (case-insensitive).
pub fn all() -> Vec<Arc<dyn Provider>> {
    let mut providers: Vec<Arc<dyn Provider>> = vec![
        Arc::new(AnonTransfer::default()),
        Arc::new(Anonymfile),
        Arc::new(BowFile),
        Arc::new(Buzzheavier),
        Arc::new(Catbox),
        Arc::new(Delafil),
        Arc::new(DooDrive),
        Arc::new(DownloadGg),
        Arc::new(EasyUpload::default()),
        Arc::new(Fastupload::default()),
        Arc::new(FileBin),
        Arc::new(FileIo),
        Arc::new(FilesDpUa),
        Arc::new(FilesFm),
        Arc::new(FileSi),
        Arc::new(FileUpload),
        Arc::new(Gofile::default()),
        Arc::new(Keep),
        Arc::new(Krakenfiles),
        Arc::new(LitterBox),
        Arc::new(Mixdrop),
        Arc::new(OneFichier),
        Arc::new(Oshi),
        Arc::new(Pixeldrain),
        Arc::new(Rapidgator::default()),
        Arc::new(TempSend),
        Arc::new(TmpFiles),
        Arc::new(TransferSh),
        Arc::new(UFile::default()),
    ];
    providers.extend(
        jqfu::providers()
            .into_iter()
            .map(|s| Arc::new(s) as Arc<dyn Provider>),
    );
    providers.extend(
        xfs::providers()
            .into_iter()
            .map(|s| Arc::new(s) as Arc<dyn Provider>),
    );
    providers.sort_by_key(|provider| provider.info().name.to_lowercase());
    providers
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::provider::{Auth, normalize_name};

    #[test]
    fn every_legacy_provider_is_registered() {
        // The 37 providers of the Python version (`modules/provider_data.py` at tag 1.5.2).
        let legacy = [
            "Gofile",
            "Pixeldrain",
            "FileBin",
            "Delafil",
            "Files.dp.ua",
            "FilesFm",
            "Krakenfiles",
            "Transfer_sh",
            "TmpFiles",
            "Mixdrop",
            "1Fichier",
            "Fileio",
            "EasyUpload",
            "AnonTransfer",
            "1CloudFile",
            "Anonymfile",
            "FileSi",
            "ClicknUpload",
            "FileUpload",
            "BowFile",
            "HexUpload",
            "UserCloud",
            "DooDrive",
            "uFile",
            "Download_gg",
            "Catbox",
            "LitterBox",
            "Keep",
            "TempSend",
            "UsersDrive",
            "Rapidgator",
            "WDHO",
            "Filesadmin",
            "Fastupload",
            "CyberFile",
            "Buzzheavier",
            "Oshi",
        ];
        assert_eq!(legacy.len(), 37);
        let registry = crate::Registry::builtin();
        for name in legacy {
            assert!(registry.find(name).is_some(), "{name} is not registered");
        }
        assert_eq!(registry.len(), 37);
    }

    #[test]
    fn names_and_aliases_are_unique() {
        let mut seen = HashSet::new();
        for provider in all() {
            let info = provider.info();
            for name in std::iter::once(info.name).chain(info.aliases.iter().copied()) {
                assert!(
                    seen.insert(normalize_name(name)),
                    "`{name}` is used by more than one provider"
                );
            }
        }
    }

    #[test]
    fn registry_is_sorted() {
        let names: Vec<String> = all().iter().map(|s| s.info().name.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn provider_metadata_is_consistent() {
        for provider in all() {
            let info = provider.info();
            assert!(info.homepage.starts_with("https://"), "{}", info.name);
            assert!(info.probe_url.starts_with("https://"), "{}", info.name);
            match info.auth {
                Auth::None => assert!(info.credentials.is_empty(), "{}", info.name),
                _ => assert!(!info.credentials.is_empty(), "{}", info.name),
            }
            for ext in info.blocked_extensions {
                assert!(
                    ext.starts_with('.') && *ext == ext.to_lowercase(),
                    "{}",
                    info.name
                );
            }
        }
    }

    #[test]
    fn exactly_the_legacy_credential_providers_need_credentials() {
        let mut keyed: Vec<_> = all()
            .iter()
            .filter(|s| s.info().auth == Auth::Required)
            .map(|s| s.info().name)
            .collect();
        keyed.sort();
        assert_eq!(
            keyed,
            ["BowFile", "DooDrive", "Mixdrop", "Pixeldrain", "Rapidgator"]
        );
    }
}
