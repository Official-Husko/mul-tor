//! The set of providers the program knows about.

use std::fmt;
use std::sync::Arc;

use super::Provider;

/// All providers the program knows about.
#[derive(Clone)]
pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    pub fn new(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self { providers }
    }

    /// Every provider shipped with Mul-Tor.
    pub fn builtin() -> Self {
        Self::new(crate::providers::all())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.providers.iter()
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn find(&self, query: &str) -> Option<&Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|provider| provider.info().matches(query))
    }

    /// Looks every name up; returns the providers found (in the order given,
    /// without duplicates) and the names that matched nothing.
    pub fn resolve<'a, I>(&self, names: I) -> (Vec<Arc<dyn Provider>>, Vec<String>)
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut found: Vec<Arc<dyn Provider>> = Vec::new();
        let mut unknown = Vec::new();
        for name in names {
            match self.find(name) {
                Some(provider) => {
                    if !found.iter().any(|f| Arc::ptr_eq(f, provider)) {
                        found.push(provider.clone());
                    }
                }
                None => unknown.push(name.to_string()),
            }
        }
        (found, unknown)
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.providers.iter().map(|s| s.info().name))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::error::UploadError;
    use crate::provider::ProviderInfo;
    use crate::upload::{UploadContext, UploadedFile};

    struct Stub(ProviderInfo);

    #[async_trait]
    impl Provider for Stub {
        fn info(&self) -> &ProviderInfo {
            &self.0
        }

        async fn upload(&self, _: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
            unreachable!("not uploaded in these tests")
        }
    }

    fn registry() -> Registry {
        Registry::new(vec![
            Arc::new(Stub(ProviderInfo::new("Alpha", "", "").aliases(&["A_one"]))),
            Arc::new(Stub(ProviderInfo::new("Beta", "", ""))),
        ])
    }

    #[test]
    fn resolve_keeps_order_drops_duplicates_and_reports_unknown_names() {
        let (found, unknown) = registry().resolve(["beta", "A.one", "BETA", "nope"]);
        let names: Vec<_> = found.iter().map(|p| p.info().name).collect();
        assert_eq!(names, ["Beta", "Alpha"]);
        assert_eq!(unknown, ["nope"]);
    }
}
