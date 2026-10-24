//! The built-in sites.
//!
//! Adding a site: see `docs/ADDING_SITES.md`.
//!
//! Sites that share a protocol are declared as data in a family module
//! (`xfs`, `jqfu`); the rest get a file of their own. The Dropzone helper is
//! used by several of those individual files.

use std::sync::Arc;

use crate::site::Site;

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
mod fileio;
mod filebin;
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
pub use jqfu::JqfuSite;
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
pub use xfs::XfsSite;

/// Every built-in site, sorted by name (case-insensitive).
pub fn all() -> Vec<Arc<dyn Site>> {
    let mut sites: Vec<Arc<dyn Site>> = vec![
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
    sites.extend(jqfu::sites().into_iter().map(|s| Arc::new(s) as Arc<dyn Site>));
    sites.extend(xfs::sites().into_iter().map(|s| Arc::new(s) as Arc<dyn Site>));
    sites.sort_by_key(|site| site.info().name.to_lowercase());
    sites
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::site::{Auth, normalize_name};

    #[test]
    fn every_legacy_site_is_registered() {
        // The 37 sites of the Python version (`modules/site_data.py`).
        let legacy = [
            "Gofile", "Pixeldrain", "FileBin", "Delafil", "Files.dp.ua", "FilesFm", "Krakenfiles",
            "Transfer_sh", "TmpFiles", "Mixdrop", "1Fichier", "Fileio", "EasyUpload", "AnonTransfer",
            "1CloudFile", "Anonymfile", "FileSi", "ClicknUpload", "FileUpload", "BowFile",
            "HexUpload", "UserCloud", "DooDrive", "uFile", "Download_gg", "Catbox", "LitterBox",
            "Keep", "TempSend", "UsersDrive", "Rapidgator", "WDHO", "Filesadmin", "Fastupload",
            "CyberFile", "Buzzheavier", "Oshi",
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
        for site in all() {
            let info = site.info();
            for name in std::iter::once(info.name).chain(info.aliases.iter().copied()) {
                assert!(
                    seen.insert(normalize_name(name)),
                    "`{name}` is used by more than one site"
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
    fn site_metadata_is_consistent() {
        for site in all() {
            let info = site.info();
            assert!(info.homepage.starts_with("https://"), "{}", info.name);
            assert!(info.probe_url.starts_with("https://"), "{}", info.name);
            match info.auth {
                Auth::None => assert!(info.credentials.is_empty(), "{}", info.name),
                _ => assert!(!info.credentials.is_empty(), "{}", info.name),
            }
            for ext in info.blocked_extensions {
                assert!(ext.starts_with('.') && *ext == ext.to_lowercase(), "{}", info.name);
            }
        }
    }

    #[test]
    fn exactly_the_legacy_credential_sites_need_credentials() {
        let mut keyed: Vec<_> = all()
            .iter()
            .filter(|s| s.info().auth == Auth::Required)
            .map(|s| s.info().name)
            .collect();
        keyed.sort();
        assert_eq!(keyed, ["BowFile", "DooDrive", "Mixdrop", "Pixeldrain", "Rapidgator"]);
    }
}
