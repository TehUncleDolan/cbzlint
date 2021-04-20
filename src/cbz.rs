//! CBZ check implementation.

use crate::{
    bedetheque,
    error::Error,
};
use anyhow::{
    bail,
    Context,
    Result,
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    ffi::OsStr,
    fs,
    path::{
        Path,
        PathBuf,
    },
};
use zip::{
    read::ZipFile,
    ZipArchive,
};

/// Regex to extract info from the name of a series' book.
static SERIES_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
            r#"^(?P<title>.+)(?: T(?P<volume>[0-9]+)) \((?P<authors>.+)\) \((?P<year>[0-9]{4})\) \[Digital-(?P<width>[0-9]+)\]"#,
        )
        .expect("valid series regexp")
});

/// Regex to extract info from the name of a one-shot.
static ONESHOT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
            r#"^(?P<title>.+) \((?P<authors>.+)\) \((?P<year>[0-9]{4})\) \[Digital-(?P<width>[0-9]+)\]"#,
        )
        .expect("valid one-shot regexp")
});

#[derive(Debug)]
pub(crate) struct Book {
    path: PathBuf,
    title: String,
    volume: Option<u8>,
    authors: String,
    year: u16,
    width: usize,
}

impl Book {
    /// Initialize a new book by extracting information from its name.
    pub(crate) fn new(path: &Path) -> Result<Self> {
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

        Ok(Self::new_from_captures(path.to_owned(), &captures))
    }

    /// Return the file name of the book.
    pub(crate) fn file_name(&self) -> &str {
        get_file_name(&self.path)
    }

    /// Check the book and return a list of errors if any.
    pub(crate) fn check(
        &self,
        client: &bedetheque::Client,
    ) -> Result<Vec<Error>> {
        let mut errors = Vec::new();
        let fp = fs::File::open(&self.path).context("open error")?;
        let mut cbz = ZipArchive::new(fp).context("read error")?;

        self.check_book_metadata(client, &mut errors)?;
        for i in 0..cbz.len() {
            let mut entry =
                cbz.by_index(i).context("failed to read ZIP entry")?;

            if !entry.is_file() {
                continue;
            }

            self.check_width(&mut entry, &mut errors)?;
        }

        Ok(errors)
    }

    fn new_from_captures(
        path: PathBuf,
        captures: &regex::Captures<'_>,
    ) -> Self {
        Self {
            path,
            title: captures
                .name("title")
                .expect("invalid capture group for title")
                .as_str()
                .to_owned(),
            volume: captures
                .name("volume")
                .map(|m| m.as_str().parse::<u8>().expect("valid volume")),
            authors: captures
                .name("authors")
                .expect("invalid capture group for authors")
                .as_str()
                .to_owned(),
            year: captures
                .name("year")
                .expect("invalid capture group for year")
                .as_str()
                .parse::<u16>()
                .expect("valid year"),
            width: captures
                .name("width")
                .expect("invalid capture group for width")
                .as_str()
                .parse::<usize>()
                .expect("valid width"),
        }
    }

    /// Check that the width of every image match the name.
    ///
    /// Width must be equal (single page) or twice as large (dual page).
    ///
    /// Returns one error per page with an invalid width.
    fn check_width(
        &self,
        entry: &mut ZipFile<'_>,
        errors: &mut Vec<Error>,
    ) -> Result<()> {
        let mut bytes: Vec<u8> = vec![];
        std::io::copy(entry, &mut bytes).with_context(|| {
            format!("failed to read image {}", entry.name())
        })?;
        let width = imagesize::blob_size(&bytes)
            .with_context(|| format!("cannot get width for {}", entry.name()))?
            .width;

        if width != self.width && width != 2 * self.width {
            errors.push(Error::Width {
                page: get_file_name(Path::new(entry.name())).to_owned(),
                width,
            })
        }

        Ok(())
    }

    /// Check the book's metadata (authors, publication years, ...)
    fn check_book_metadata(
        &self,
        client: &bedetheque::Client,
        errors: &mut Vec<Error>,
    ) -> Result<()> {
        let info = client
            .get_book_info(&self.title, self.volume)
            .context("failed to get metadata from bedetheque")?;

        if info.authors != self.authors {
            errors.push(Error::Authors(info.authors));
        }

        if !info.years.contains(&self.year) {
            errors.push(Error::Year(info.years));
        }

        Ok(())
    }
}

/// Extract the file name, as UTF-8 string, from a file path.
fn get_file_name(path: &Path) -> &str {
    path.file_name()
        .expect("filename")
        .to_str()
        .expect("valid UTF-8")
}
