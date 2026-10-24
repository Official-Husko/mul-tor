//! The built-in providers.
//!
//! Adding a provider: see `docs/ADDING_PROVIDERS.md`.
//!
//! Providers that share a protocol are declared as data in a family module
//! (`xfs`, `jqfu`, `form_post`); the rest get a file of their own. The Dropzone helper is
//! used by several of those individual files.

use std::sync::Arc;

use crate::provider::Provider;

mod dropzone;
mod form_post;
mod jqfu;
mod xfs;

mod anontransfer;
mod anonymfile;
mod bowfile;
mod buzzheavier;
mod catbox;
mod curlby;
mod delafil;
mod doodrive;
mod downloadgg;
mod easyupload;
mod fastupload;
mod filebin;
mod fileio;
mod filemirage;
mod filesdpua;
mod filesfm;
mod filesi;
mod fileupload;
mod gigafile;
mod gofile;
mod keep;
mod krakenfiles;
mod mixdrop;
mod onefichier;
mod oshi;
mod pixeldrain;
mod rapidgator;
mod storage_to;
mod tempsend;
mod tmpfiles;
mod transfersh;
mod ufile;
mod vikingfile;

pub use anontransfer::AnonTransfer;
pub use anonymfile::Anonymfile;
pub use bowfile::BowFile;
pub use buzzheavier::Buzzheavier;
pub use catbox::{Catbox, LitterBox};
pub use curlby::CurlBy;
pub use delafil::Delafil;
pub use doodrive::DooDrive;
pub use downloadgg::DownloadGg;
pub use easyupload::EasyUpload;
pub use fastupload::Fastupload;
pub use filebin::FileBin;
pub use fileio::FileIo;
pub use filemirage::FileMirage;
pub use filesdpua::FilesDpUa;
pub use filesfm::FilesFm;
pub use filesi::FileSi;
pub use fileupload::FileUpload;
pub use form_post::FormPostProvider;
pub use gigafile::GigaFile;
pub use gofile::Gofile;
pub use jqfu::JqfuProvider;
pub use keep::Keep;
pub use krakenfiles::Krakenfiles;
pub use mixdrop::Mixdrop;
pub use onefichier::OneFichier;
pub use oshi::Oshi;
pub use pixeldrain::Pixeldrain;
pub use rapidgator::Rapidgator;
pub use storage_to::StorageTo;
pub use tempsend::TempSend;
pub use tmpfiles::TmpFiles;
pub use transfersh::TransferSh;
pub use ufile::UFile;
pub use vikingfile::VikingFile;
pub use xfs::XfsProvider;

/// Every built-in provider, sorted by name (case-insensitive).
pub fn all() -> Vec<Arc<dyn Provider>> {
    let mut providers: Vec<Arc<dyn Provider>> = vec![
        Arc::new(AnonTransfer::default()),
        Arc::new(Anonymfile),
        Arc::new(BowFile),
        Arc::new(Buzzheavier),
        Arc::new(Catbox),
        Arc::new(CurlBy),
        Arc::new(Delafil),
        Arc::new(DooDrive),
        Arc::new(DownloadGg),
        Arc::new(EasyUpload::default()),
        Arc::new(Fastupload::default()),
        Arc::new(FileBin),
        Arc::new(FileIo),
        Arc::new(FileMirage::default()),
        Arc::new(FilesDpUa),
        Arc::new(FilesFm),
        Arc::new(FileSi),
        Arc::new(FileUpload),
        Arc::new(GigaFile::default()),
        Arc::new(Gofile::default()),
        Arc::new(Keep),
        Arc::new(Krakenfiles),
        Arc::new(LitterBox),
        Arc::new(Mixdrop),
        Arc::new(OneFichier),
        Arc::new(Oshi),
        Arc::new(Pixeldrain),
        Arc::new(Rapidgator::default()),
        Arc::new(StorageTo),
        Arc::new(TempSend),
        Arc::new(TmpFiles),
        Arc::new(TransferSh),
        Arc::new(UFile::default()),
        Arc::new(VikingFile),
    ];
    providers.extend(
        form_post::providers()
            .into_iter()
            .map(|s| Arc::new(s) as Arc<dyn Provider>),
    );
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
mod tests;
