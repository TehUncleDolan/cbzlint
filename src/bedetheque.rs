//! HTTP client to retrieve information from bedetheque.

use crate::metadata::VolumeInfo;
use anyhow::{anyhow, Context, Result};
use kuchiki::traits::*;
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap, thread, time::Duration};
use url::Url;

/// Bedetheque homepage.
static MAIN_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://www.bedetheque.com/").expect("valid homepage URL"));

/// Bedetheque URL where to submit search form..
static SEARCH_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://www.bedetheque.com/search/albums").expect("valid search URL"));

/// CSS selector to extract CSRF token from the form's home page.
static CSRF_TOKEN_SELECTOR: Lazy<kuchiki::Selectors> =
    Lazy::new(|| kuchiki::Selectors::compile("#csrf").expect("invalid CSRF token selector"));

/// CSS selector to extract the book URLs from the search result page.
static LINKS_SELECTOR: Lazy<kuchiki::Selectors> =
    Lazy::new(|| kuchiki::Selectors::compile(".search-list li a").expect("invalid links selector"));

/// CSS selector to extract the series title.
static TITLE_SELECTOR: Lazy<kuchiki::Selectors> =
    Lazy::new(|| kuchiki::Selectors::compile(".serie").expect("invalid title selector"));

/// CSS selector to extract the volume number, if any.
static VOLUME_SELECTOR: Lazy<kuchiki::Selectors> =
    Lazy::new(|| kuchiki::Selectors::compile(".num").expect("invalid volume selector"));

/// A volume identifier, used as cache key.
#[derive(Debug, Eq, Hash, PartialEq)]
struct Volume {
    title: String,
    // Optional because One-Shot don't have one.
    volume: Option<u8>,
}

/// A bedetheque client.
pub(crate) struct Client {
    agent: ureq::Agent,
    cache: RefCell<HashMap<Volume, Url>>,
}

impl Client {
    /// Initialize a new Bedetheque client.
    pub(crate) fn new() -> Self {
        Self {
            agent: ureq::Agent::new(),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Find the book's URL on bedetheque.
    pub(crate) fn find_book(&self, title: &str, volume: Option<u8>) -> Result<Url> {
        let key = Volume {
            title: title.to_owned(),
            volume,
        };

        if let Some(url) = self.cache.borrow().get(&key) {
            return Ok(url.clone());
        }

        let csrf_token = self.get_csrf_token()?;
        let mut url = SEARCH_URL.clone();
        url.query_pairs_mut()
            .append_pair("csrf_token_bel", &csrf_token)
            .append_pair("RechSerie", &normalize(title))
            .append_pair("RechLangue", "Français");

        let mut res = self.get_link(title, volume, &url);

        // No result with hyphen, try without it then!
        if res.is_err() && title.contains('-') {
            let title = title.replace("- ", "");

            url = SEARCH_URL.clone();
            url.query_pairs_mut()
                .append_pair("csrf_token_bel", &csrf_token)
                .append_pair("RechSerie", &normalize(&title))
                .append_pair("RechLangue", "Français");

            res = self.get_link(&title, volume, &url);
        }

        res
    }

    /// Extract metadata from the book's page.
    pub(crate) fn fetch_info(&self, url: &Url) -> Result<VolumeInfo> {
        let html = self.get_html(url)?;

        Ok(VolumeInfo::new(&html))
    }

    /// Extract the CSRF token from the homepage.
    #[allow(clippy::filter_next)]
    fn get_csrf_token(&self) -> Result<String> {
        let html = self.get_html(&MAIN_URL)?;

        Ok(CSRF_TOKEN_SELECTOR
            .filter(html.descendants().elements())
            .next()
            .context("CSRF token not found")?
            .attributes
            .borrow()
            .get("value")
            .context("CSRF token missing")?
            .to_owned())
    }

    /// Get the book's URLs from the search result of a the given series.
    fn get_link(&self, title: &str, volume: Option<u8>, url: &Url) -> Result<Url> {
        let mut res = None;

        let html = self.get_html(url)?;
        // First, look for an exact match.
        let mut nodes = LINKS_SELECTOR
            .filter(html.descendants().elements())
            .filter(|element| is_right_series(element.as_node(), title, true))
            .collect::<Vec<_>>();
        // If none are found, fallback on prefix then...
        if nodes.is_empty() {
            nodes = LINKS_SELECTOR
                .filter(html.descendants().elements())
                .filter(|element| is_right_series(element.as_node(), title, false))
                .collect::<Vec<_>>();
        }

        for node in nodes {
            let attributes = node.attributes.borrow();
            let link = attributes.get("href").context("book URL not found")?;
            let url = Url::parse(link).with_context(|| format!("invalid book URL `{}`", link))?;

            let number = get_book_number(node.as_node())?;

            if number == volume {
                res = Some(url.clone());
            }

            let key = Volume {
                title: title.to_owned(),
                volume: number,
            };
            self.cache.borrow_mut().insert(key, url);
        }

        res.ok_or_else(|| anyhow!("cannot find book on bedetheque"))
    }

    /// Retrieve and parse the page at `url`.
    fn get_html(&self, url: &Url) -> Result<kuchiki::NodeRef> {
        // Don't get banned from bedetheque...
        thread::sleep(Duration::new(2, 0));

        let response = self
            .agent
            .request_url("GET", url)
            .set("accept", "text/html")
            .set("Referer", MAIN_URL.as_str())
            .call()?;

        let html = response
            .into_string()
            .with_context(|| format!("failed to read HTML from {}", url.as_str()))?;

        Ok(kuchiki::parse_html().one(html))
    }
}

/// Extract the book number, if any, from the book link.
#[allow(clippy::filter_next)]
fn get_book_number(node: &kuchiki::NodeRef) -> Result<Option<u8>> {
    let text = VOLUME_SELECTOR
        .filter(node.descendants().elements())
        .next()
        .context("book number not found")?
        .text_contents();

    if text.is_empty() {
        return Ok(None);
    }

    text.trim_start_matches('#')
        .parse::<u8>()
        .context("invalid book number")
        .map(Some)
}

/// Check if the series under `node` is the right one (i.e. matches `title`).
#[allow(clippy::filter_next)]
fn is_right_series(node: &kuchiki::NodeRef, title: &str, exact_match: bool) -> bool {
    let title = normalize(title);

    match TITLE_SELECTOR.filter(node.descendants().elements()).next() {
        Some(node) => {
            let text = node.text_contents().replace("!", "");
            let text = text.trim().to_lowercase();

            if exact_match {
                text == title
            } else {
                text.starts_with(&title)
            }
        }
        None => false,
    }
}

/// Normalize the series' title for bedetheque.
fn normalize(title: &str) -> String {
    title.to_lowercase()
}
