//! Extract book's metadata from the book's page.

use kuchiki::traits::*;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{collections::BTreeSet, iter::FromIterator};

/// CSS selector for the information fields.
static INFO_SELECTOR: Lazy<kuchiki::Selectors> =
    Lazy::new(|| kuchiki::Selectors::compile(".infos li").expect("invalid info selector"));

/// Regex to extract the writer or pencillers name.
static AUTHOR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?P<category>Scénario|Dessin) :\s+(?P<name>[^,]+)"#).expect("valid author regexp")
});

/// Regex to extract the publication year.
static YEAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Dépot légal :\s+[0-9]{2}/(?P<year>[0-9]{4})"#).expect("valid year regexp")
});

/// Volume metadata.
pub(crate) struct VolumeInfo {
    /// Authors names.
    pub(crate) authors: String,
    /// Publicaton year of every editions.
    pub(crate) years: BTreeSet<u16>,
}

impl VolumeInfo {
    pub(crate) fn new(page: &kuchiki::NodeRef) -> Self {
        let mut years = BTreeSet::new();
        let mut writers = BTreeSet::new();
        let mut pencillers = BTreeSet::new();

        for node in INFO_SELECTOR.filter(page.descendants().elements()) {
            let content = node.text_contents();

            if let Some(captures) = AUTHOR_REGEX.captures(&content) {
                let category = captures
                    .name("category")
                    .expect("invalid capture group for author's category")
                    .as_str();
                let name = captures
                    .name("name")
                    .expect("invalid capture group for author's name")
                    .as_str()
                    .trim()
                    .to_owned();

                if category == "Dessin" {
                    // Don't add the author as penciller if they are already
                    // registered as a writer.
                    //
                    // This works because writers are always listed first on the
                    // page.
                    if !writers.contains(&name) {
                        pencillers.insert(name);
                    }
                } else {
                    writers.insert(name);
                }
            } else if let Some(captures) = YEAR_REGEX.captures(&content) {
                let year = captures
                    .name("year")
                    .expect("invalid capture group for width")
                    .as_str()
                    .parse::<u16>()
                    .expect("valid year");
                years.insert(year);
            }
        }

        // Writers first, pencillers next. Already sorted alphabetically thanks
        // to the BTree.
        let mut authors = Vec::from_iter(writers);
        authors.extend(pencillers);

        Self {
            authors: authors.join("-"),
            years,
        }
    }
}
