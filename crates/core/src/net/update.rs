//! Checking GitHub for a newer release. Notification only: nothing is
//! downloaded or installed, so the program never replaces its own binary.

use std::time::Duration;

use semver::Version;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::http::Http;

const LATEST: &str = "https://api.github.com/repos/Official-Husko/mul-tor/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub tag: String,
    pub name: String,
    /// Release notes (Markdown).
    pub notes: String,
    pub published: String,
    pub url: String,
}

#[derive(Deserialize)]
struct Raw {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    html_url: String,
}

impl Release {
    fn from_raw(raw: Raw) -> Result<Self> {
        let version = parse_tag(&raw.tag_name)
            .ok_or_else(|| Error::Other(format!("unrecognised release tag `{}`", raw.tag_name)))?;
        Ok(Self {
            version,
            name: raw
                .name
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| raw.tag_name.clone()),
            tag: raw.tag_name,
            notes: raw.body.unwrap_or_default(),
            published: raw
                .published_at
                .unwrap_or_default()
                .replace('T', " ")
                .replace('Z', ""),
            url: raw.html_url,
        })
    }
}

/// `v2.0.0` / `2.0.0-alpha.1` -> version.
fn parse_tag(tag: &str) -> Option<Version> {
    Version::parse(tag.trim().trim_start_matches(['v', 'V'])).ok()
}

/// The newest published release.
pub async fn latest_release(http: &Http) -> Result<Release> {
    let response = http
        .get(LATEST)
        .timeout(TIMEOUT)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::Other(format!(
            "GitHub answered {} when asked for the latest release",
            response.status()
        )));
    }
    Release::from_raw(response.json::<Raw>().await?)
}

/// Is `release` newer than the running `current` version?
/// (The Python version compared the version strings, which breaks at 1.10.)
pub fn is_newer(current: &str, release: &Release) -> bool {
    parse_tag(current).is_some_and(|current| release.version > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release::from_raw(Raw {
            tag_name: tag.into(),
            name: None,
            body: None,
            published_at: Some("2024-07-01T10:20:30Z".into()),
            html_url: "https://example/r".into(),
        })
        .unwrap()
    }

    #[test]
    fn versions_compare_numerically_not_textually() {
        assert!(is_newer("1.9.0", &release("v1.10.0")));
        assert!(!is_newer("1.10.0", &release("v1.9.0")));
        assert!(!is_newer("1.5.2", &release("v1.5.2")));
    }

    #[test]
    fn prereleases_sort_before_their_release() {
        assert!(is_newer("2.0.0-alpha.1", &release("2.0.0-alpha.2")));
        assert!(is_newer("2.0.0-alpha.2", &release("2.0.0")));
        assert!(!is_newer("2.0.0", &release("2.0.0-rc.1")));
        assert!(is_newer("1.5.2", &release("2.0.0-alpha.1")));
    }

    #[test]
    fn unparseable_tags_are_errors_not_panics() {
        let raw = Raw {
            tag_name: "nightly".into(),
            name: None,
            body: None,
            published_at: None,
            html_url: String::new(),
        };
        assert!(Release::from_raw(raw).is_err());
        assert!(!is_newer("not a version", &release("v1.0.0")));
    }

    #[test]
    fn published_date_is_readable() {
        assert_eq!(release("v1.0.0").published, "2024-07-01 10:20:30");
        assert_eq!(release("v1.0.0").name, "v1.0.0");
    }
}
