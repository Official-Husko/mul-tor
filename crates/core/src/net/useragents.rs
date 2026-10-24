//! Browser user agents for `random_user_agent`.
//!
//! The list is downloaded and cached for ten days in `user_agents.json`, in the
//! same format the Python version wrote, so an existing cache keeps working.

use std::path::Path;

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::http::Http;

const SOURCE: &str = "https://useragents.me/api";
const MAX_AGE_DAYS: i64 = 10;

#[derive(Debug, Serialize, Deserialize)]
struct Cache {
    /// `YYYY-MM-DD`
    creation_date: String,
    user_agents: Vec<String>,
}

impl Cache {
    fn is_fresh(&self, today: NaiveDate) -> bool {
        NaiveDate::parse_from_str(&self.creation_date, "%Y-%m-%d")
            .map(|created| (today - created).num_days() <= MAX_AGE_DAYS)
            .unwrap_or(false)
            && !self.user_agents.is_empty()
    }
}

/// Used when the list cannot be downloaded and nothing is cached.
pub fn backup_list() -> Vec<String> {
    [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36 Edg/109.0.1518.70",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/109.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Safari/605.1.15",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.3 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.67 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:102.0) Gecko/20100101 Firefox/102.0",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.54 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.4896.75 Safari/537.36 Edg/100.0.1185.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36 Edg/105.0.1343.42",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// The user agents to rotate through: the cache if it is fresh, otherwise a
/// download (which refreshes the cache), otherwise a stale cache, otherwise
/// the built-in list. Never fails and never returns an empty list.
pub async fn load(cache_path: &Path, http: &Http) -> Vec<String> {
    let cached: Option<Cache> = std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());

    if let Some(cache) = &cached
        && cache.is_fresh(Local::now().date_naive())
    {
        return cache.user_agents.clone();
    }

    match download(http).await {
        Ok(list) => {
            store(cache_path, &list);
            list
        }
        Err(reason) => {
            tracing::warn!(%reason, "could not download user agents");
            cached
                .map(|c| c.user_agents)
                .filter(|list| !list.is_empty())
                .unwrap_or_else(backup_list)
        }
    }
}

async fn download(http: &Http) -> Result<Vec<String>, String> {
    let response = http.get(SOURCE).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let list: Vec<String> = json
        .get("data")
        .and_then(|d| d.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.get("ua")?.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if list.is_empty() {
        Err("the reply contained no user agents".into())
    } else {
        Ok(list)
    }
}

fn store(path: &Path, list: &[String]) {
    let cache = Cache {
        creation_date: Local::now().date_naive().to_string(),
        user_agents: list.to_vec(),
    };
    let write = || -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&cache)?)
    };
    if let Err(error) = write() {
        tracing::warn!(%error, "could not write the user agent cache");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(date: &str) -> Cache {
        Cache {
            creation_date: date.into(),
            user_agents: vec!["UA".into()],
        }
    }

    #[test]
    fn freshness_window_is_ten_days() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
        assert!(cache("2024-06-20").is_fresh(today));
        assert!(cache("2024-06-10").is_fresh(today));
        assert!(!cache("2024-06-09").is_fresh(today));
        assert!(!cache("1970-01-01").is_fresh(today));
        assert!(!cache("garbage").is_fresh(today));
    }

    #[test]
    fn an_empty_cache_is_never_fresh() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
        let empty = Cache {
            creation_date: "2024-06-20".into(),
            user_agents: vec![],
        };
        assert!(!empty.is_fresh(today));
    }

    #[test]
    fn the_python_cache_format_is_readable() {
        let text = r#"{"creation_date": "2024-06-20", "user_agents": ["A", "B"]}"#;
        let parsed: Cache = serde_json::from_str(text).unwrap();
        assert_eq!(parsed.user_agents, ["A", "B"]);
    }

    #[test]
    fn backup_list_is_not_empty() {
        assert_eq!(backup_list().len(), 15);
    }
}
