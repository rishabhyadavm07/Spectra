use reqwest::cookie::{CookieStore, Jar};
use std::sync::{Arc, RwLock};

/// A custom reqwest `CookieStore` that allows dropping all stored cookies
/// on demand by simply replacing the inner `Jar` with a fresh one.
#[derive(Clone, Default)]
pub struct ClearableCookieStore {
    inner: Arc<RwLock<Jar>>,
}

impl ClearableCookieStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        *self.inner.write().unwrap() = Jar::default();
    }
}

impl CookieStore for ClearableCookieStore {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &reqwest::header::HeaderValue>, url: &reqwest::Url) {
        self.inner.write().unwrap().set_cookies(cookie_headers, url)
    }

    fn cookies(&self, url: &reqwest::Url) -> Option<reqwest::header::HeaderValue> {
        self.inner.read().unwrap().cookies(url)
    }
}
