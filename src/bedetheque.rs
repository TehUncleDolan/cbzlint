//! HTTP client to retrieve information from bedetheque.

use crate::metadata::VolumeInfo;
use anyhow::{
    bail,
    Context,
    Result,
};
use kuchiki::traits::*;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    cell::RefCell,
    iter::Cycle,
    thread,
    time::Duration,
};
use url::Url;

/// Bedetheque homepage.
const MAIN_URL: &str = "https://www.bedetheque.com/";

const SEARCH_SCOPE: &str = "site:www.bedetheque.com";

/// CSS selector to extract the book URLs from the search result page.
static LINKS_SELECTOR: Lazy<kuchiki::Selectors> = Lazy::new(|| {
    kuchiki::Selectors::compile(".result_header a")
        .expect("invalid links selector")
});

/// Regex matching characters we want to strip from the title.
static SPECIAL_CHARS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"[#-+!]"#).expect("valid chars regex"));

/// Regex to look for manga format on the book's page.
static MANGA_FORMAT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Format\s+:\s+Format Manga"#).expect("valid format regex")
});

/// Regex to look for volume number on the book's page.
static MANGA_VOLUME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Tome\s+:\s+(?P<number>[0-9]+)"#).expect("valid volume regex")
});

/// A bedetheque client.
pub(crate) struct Client<'a> {
    agent: ureq::Agent,
    servers: RefCell<Cycle<std::slice::Iter<'a, Url>>>,
}

impl<'a> Client<'a> {
    /// Initialize a new Bedetheque client.
    pub(crate) fn new(serverlist: &'a [Url]) -> Self {
        Self {
            agent: ureq::Agent::new(),
            servers: RefCell::new(serverlist.iter().cycle()),
        }
    }

    /// Get the book's metadata.
    pub(crate) fn fetch_info(
        &self,
        title: &str,
        volume: Option<u8>,
    ) -> Result<VolumeInfo> {
        // Don't get blocked...
        thread::sleep(Duration::new(2, 0));

        let title = SPECIAL_CHARS.replace_all(title, "");
        let query = if let Some(number) = volume {
            format!("{} {} {}", title, number, SEARCH_SCOPE)
        } else {
            format!("{} {}", title, SEARCH_SCOPE)
        };
        let (server, html) = self.search(&query)?;

        for element in LINKS_SELECTOR.filter(html.descendants().elements()) {
            let attributes = element.attributes.borrow();
            let link = attributes.get("href").context("book URL not found")?;
            let mut url = Url::parse(link)
                .with_context(|| format!("invalid book URL `{}`", link))?;

            // Force hostname to avoid the mobile version.
            url.set_host(Some("www.bedetheque.com"))
                .expect("valid hostname");

            // Bedetheque's page for book starts with this prefix.
            if !url.path().starts_with("/BD-") {
                continue;
            }
            let page = self.get_html(&url)?;

            if is_right_book(&page, volume) {
                return Ok(VolumeInfo::new(url, &page));
            }
        }

        bail!("cannot find book on bedetheque from {}", server.as_str());
    }

    /// Retrieve and parse the page at `url`.
    fn get_html(&self, url: &Url) -> Result<kuchiki::NodeRef> {
        let response = self
            .agent
            .request_url("GET", url)
            .set("accept", "text/html")
            .set("Referer", MAIN_URL)
            .call()?;

        let html = response.into_string().with_context(|| {
            format!("failed to read HTML from {}", url.as_str())
        })?;

        Ok(kuchiki::parse_html().one(html))
    }

    fn search(&self, query: &str) -> Result<(Url, kuchiki::NodeRef)> {
        let mut url = self.servers.borrow_mut().next().expect("server URL");
        let mut i = 0;

        loop {
            i += 1;

            let res = self
                .agent
                .request_url("POST", url)
                .set("accept", "text/html")
                .set("Accept-Language", "fr")
                .send_form(&[("q", query), ("language", "fr")]);

            if res.is_err() {
                // Move to another server if we still have some retry left.
                if i <= 10 {
                    url = self.servers.borrow_mut().next().expect("server URL");
                    continue;
                }
            }

            return res.context("search failed").and_then(|response| {
                let html = response.into_string().with_context(|| {
                    format!(
                        "failed to read search result from {}",
                        url.as_str()
                    )
                })?;

                Ok((url.clone(), kuchiki::parse_html().one(html)))
            });
        }
    }
}

/// Check if the page is the right one.
fn is_right_book(page: &kuchiki::NodeRef, volume: Option<u8>) -> bool {
    // Using CSS selector would be cleaner but I'm lazy today...
    let html = page.text_contents();

    let mut result = MANGA_FORMAT_REGEX.is_match(&html);

    if let Some(number) = volume {
        if let Some(captures) = MANGA_VOLUME_REGEX.captures(&html) {
            let book_volume = captures
                .name("number")
                .expect("invalid capture group for volume")
                .as_str()
                .parse::<u8>()
                .expect("valid volume");

            result = result && (book_volume == number);
        } else {
            result = false;
        }
    }

    result
}
