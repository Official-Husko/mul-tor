//! Shared fixtures for the output tests.

use mul_tor_core::{JobOutcome, UploadError, UploadedFile};

pub fn outcome(result: Result<UploadedFile, UploadError>) -> JobOutcome {
    JobOutcome {
        job: 0,
        file_name: "a \"b\".zip".into(),
        provider: "Pixeldrain",
        result,
    }
}
