//! Cookies carried from one reply to the next request.
//!
//! The HTTP client keeps no cookies of its own (most providers need none), so a
//! provider whose server ties several requests together by cookie keeps them in
//! one of these.

use crate::reply::Reply;

#[derive(Default)]
pub(crate) struct CookieJar {
    cookies: Vec<(String, String)>,
}

impl CookieJar {
    /// Remember the cookies `reply` sets. One with the name of a cookie already
    /// held replaces it; a reply that sets none changes nothing.
    pub(crate) fn absorb(&mut self, reply: &Reply) {
        for (name, value) in &reply.cookies {
            match self.cookies.iter_mut().find(|(held, _)| held == name) {
                Some(entry) => entry.1 = value.clone(),
                None => self.cookies.push((name.clone(), value.clone())),
            }
        }
    }

    /// The value for a `Cookie` header, or `None` when nothing was set yet.
    pub(crate) fn header(&self) -> Option<String> {
        if self.cookies.is_empty() {
            return None;
        }
        let pairs: Vec<String> = self
            .cookies
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        Some(pairs.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use reqwest::header::HeaderMap;

    use super::*;

    fn reply_setting(cookies: &[(&str, &str)]) -> Reply {
        Reply {
            status: StatusCode::OK,
            url: "https://example.com/".parse().unwrap(),
            headers: HeaderMap::new(),
            cookies: cookies
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
            body: String::new(),
        }
    }

    #[test]
    fn an_empty_jar_sends_no_header() {
        assert_eq!(CookieJar::default().header(), None);
    }

    #[test]
    fn cookies_accumulate_and_a_newer_value_replaces_the_older() {
        let mut jar = CookieJar::default();
        jar.absorb(&reply_setting(&[("sid", "1"), ("node", "a")]));
        assert_eq!(jar.header().as_deref(), Some("sid=1; node=a"));

        jar.absorb(&reply_setting(&[]));
        assert_eq!(jar.header().as_deref(), Some("sid=1; node=a"));

        jar.absorb(&reply_setting(&[("sid", "2")]));
        assert_eq!(jar.header().as_deref(), Some("sid=2; node=a"));
    }
}
