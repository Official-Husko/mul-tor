//! One finished upload as a line of JSON, for scripts.

use mul_tor_core::JobOutcome;
use serde_json::json;

pub fn json_line(outcome: &JobOutcome) -> String {
    let value = match &outcome.result {
        Ok(uploaded) => json!({
            "file": outcome.file_name,
            "provider": outcome.provider,
            "status": "ok",
            "url": uploaded.url,
        }),
        Err(error) => json!({
            "file": outcome.file_name,
            "provider": outcome.provider,
            "status": "error",
            "kind": error.kind(),
            "error": error.to_string(),
        }),
    };
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::testing::outcome;
    use mul_tor_core::{UploadError, UploadedFile};

    #[test]
    fn json_lines_for_success() {
        let line = json_line(&outcome(Ok(UploadedFile::new("https://x/1"))));
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["url"], "https://x/1");
        assert_eq!(value["file"], "a \"b\".zip");
        assert!(!line.contains('\n'));
    }

    #[test]
    fn json_lines_for_failure_carry_a_stable_kind() {
        let line = json_line(&outcome(Err(UploadError::TooLarge {
            limit: "20 GB".into(),
        })));
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["kind"], "too_large");
        assert!(value["error"].as_str().unwrap().contains("20 GB"));
    }
}
