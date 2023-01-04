//! CBZ check implementation.

use crate::{bedetheque, error::Error};
use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    ffi::OsStr,
    fs,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
};
use url::Url;
use zip::{read::ZipFile, DateTime, ZipArchive};

/// Regex to extract info from the name of a series' book.
static SERIES_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
            r#"^(?P<title>.+)(?: T(?P<volume>[0-9]+)) \((?P<authors>.+)\) \((?P<year>[0-9]{4})\) \[\w+-(?P<width>[0-9]+)\]"#,
        )
        .expect("valid series regexp")
});

/// Regex to extract info from the name of a one-shot.
static ONESHOT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^(?P<title>.+) \((?P<authors>.+)\) \((?P<year>[0-9]{4})\) \[\w+-(?P<width>[0-9]+)\]"#,
    )
    .expect("valid one-shot regexp")
});

/// Expected modified date.
static EXPECTED_DATE: Lazy<DateTime> =
    Lazy::new(|| DateTime::from_date_and_time(2000, 1, 1, 0, 0, 1).expect("valid date"));

#[derive(Debug)]
pub(crate) struct Book {
    path: PathBuf,
    url: Url,
    authors: String,
    year: u16,
    width: usize,
}

impl Book {
    /// Initialize a new book by extracting information from its name.
    pub(crate) fn new(client: &bedetheque::Client, path: &Path) -> Result<Self> {
        let filename = get_file_name(path);

        if path.extension() != Some(OsStr::new("cbz")) {
            bail!("not a CBZ")
        }

        let captures = if let Some(captures) = SERIES_REGEX.captures(filename) {
            captures
        } else if let Some(captures) = ONESHOT_REGEX.captures(filename) {
            captures
        } else {
            bail!("cannot extract info from filename")
        };

        Self::new_from_captures(client, path.to_owned(), &captures)
    }

    /// Return the file name of the book.
    pub(crate) fn file_name(&self) -> &str {
        get_file_name(&self.path)
    }

    /// Return the bedetheque URL used to check the metadata.
    pub(crate) fn ref_url(&self) -> &Url {
        &self.url
    }

    /// Check the book and return a list of errors if any.
    pub(crate) fn check(&self, client: &bedetheque::Client) -> Result<Vec<Error>> {
        let mut errors = Vec::new();
        let fp = fs::File::open(&self.path).context("open error")?;
        let mut cbz = ZipArchive::new(fp).context("read error")?;

        self.check_book_metadata(client, &mut errors)?;
        for i in 0..cbz.len() {
            let mut entry = cbz.by_index(i).context("failed to read ZIP entry")?;

            if !entry.is_file() {
                continue;
            }

            if !check_date(entry.last_modified()) {
                errors.push(Error::Date);
                // We found an error, we can stop here.
                break;
            }
            if !self.check_image(&mut entry, &mut errors)? {
                // We found an error, we can stop here.
                break;
            }
        }

        Ok(errors)
    }

    fn new_from_captures(
        client: &bedetheque::Client,
        path: PathBuf,
        captures: &regex::Captures<'_>,
    ) -> Result<Self> {
        let title = captures
            .name("title")
            .expect("invalid capture group for title")
            .as_str()
            .to_owned();
        let volume = captures
            .name("volume")
            .map(|m| m.as_str().parse::<u8>().expect("valid volume"));
        let authors = captures
            .name("authors")
            .expect("invalid capture group for authors")
            .as_str()
            .to_owned();
        let year = captures
            .name("year")
            .expect("invalid capture group for year")
            .as_str()
            .parse::<u16>()
            .expect("valid year");
        let width = captures
            .name("width")
            .expect("invalid capture group for width")
            .as_str()
            .parse::<usize>()
            .expect("valid width");
        let url = client.find_book(&title, volume)?;

        Ok(Self {
            path,
            url,
            authors,
            year,
            width,
        })
    }

    /// Check the image.
    ///
    /// Ensure that the width of every image match the name.
    ///
    /// Width must be equal (single page) or more or less twice as large (dual
    /// page).
    ///
    /// Also check the presence of EXIF metadata.
    fn check_image(&self, entry: &mut ZipFile<'_>, errors: &mut Vec<Error>) -> Result<bool> {
        let mut bytes: Vec<u8> = vec![];
        std::io::copy(entry, &mut bytes)
            .with_context(|| format!("failed to read image {}", entry.name()))?;

        // Check width.
        // DPR are sometimes edited, so allows 10% of variation.
        let margin = self.width / 10;
        let dpr_range = (2 * self.width - margin)..=(2 * self.width + margin);
        let width = imagesize::blob_size(&bytes)
            .with_context(|| format!("cannot get width for {}", entry.name()))?
            .width;

        if width != self.width && !dpr_range.contains(&width) {
            errors.push(Error::Width);
            return Ok(false);
        }

        // Check EXIF.
        let mut reader = BufReader::new(Cursor::new(&*bytes));
        let exifreader = exif::Reader::new();
        match exifreader.read_from_container(&mut reader) {
            Ok(_) => {
                errors.push(Error::Exif);
                Ok(false)
            }
            Err(exif::Error::NotFound(_)) => Ok(true),
            Err(err) => Err(err).with_context(|| format!("cannot check EXIF for {}", entry.name())),
        }
    }

    /// Check the book's metadata (authors, publication years, ...)
    fn check_book_metadata(
        &self,
        client: &bedetheque::Client,
        errors: &mut Vec<Error>,
    ) -> Result<()> {
        let info = client
            .fetch_info(&self.url)
            .context("failed to get metadata from bedetheque")?;

        if normalize(&info.authors) != normalize(&self.authors) {
            errors.push(Error::Authors(info.authors));
        }

        if !info.years.contains(&self.year) {
            errors.push(Error::Year(info.years));
        }

        Ok(())
    }
}

/// Check that the date match the expected one.
fn check_date(date: DateTime) -> bool {
    // Only check date, not time (weird issues for some Windows users).
    EXPECTED_DATE.year() == date.year()
        && EXPECTED_DATE.month() == date.month()
        && EXPECTED_DATE.day() == date.day()
}

/// Extract the file name, as UTF-8 string, from a file path.
fn get_file_name(path: &Path) -> &str {
    path.file_name()
        .expect("filename")
        .to_str()
        .expect("valid UTF-8")
}

/// Normalize authors list for easier comparison, best effort...
fn normalize(authors: &str) -> String {
    authors
        // Case insensitive.
        .to_lowercase()
        // Romanization mismatch.
        .replace("ā", "aa")
        .replace("â", "aa")
        .replace("ū", "uu")
        .replace("û", "uu")
        .replace("ē", "ee")
        .replace("ê", "ee")
        .replace("ō", "ou")
        .replace("ô", "ou")
        .replace("oo", "ou")
}
