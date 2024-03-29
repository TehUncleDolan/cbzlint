//! A CBZ checker.

// Lints {{{

#![deny(
    nonstandard_style,
    rust_2018_idioms,
    future_incompatible,
    rustdoc::all,
    missing_crate_level_docs,
    missing_docs,
    unreachable_pub,
    unsafe_code,
    unused,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    variant_size_differences,
    warnings,
    clippy::all,
    clippy::pedantic,
    clippy::clone_on_ref_ptr,
    clippy::exit,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::lossy_float_literal,
    clippy::mem_forget,
    clippy::panic,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::unneeded_field_pattern,
    clippy::verbose_file_reads,
    clippy::dbg_macro,
    clippy::let_underscore_must_use,
    clippy::todo,
    clippy::unwrap_used,
    clippy::use_debug
)]
#![allow(
    // The 90’s called and wanted their charset back :p
    clippy::non_ascii_literal,
    // For Kuchiki imports.
    clippy::wildcard_imports,
    // It's easily outdated and doesn't bring that much value.
    clippy::missing_errors_doc,
    // That's OK for this script.
    clippy::expect_used,
    clippy::print_stdout,
)]

// }}}

use anyhow::{Context, Result};
use std::{env, fs, path::Path};

mod bedetheque;
mod cbz;
mod error;
mod metadata;
mod termio;

fn main() -> Result<()> {
    // Setup the bedetheque client.
    let client = bedetheque::Client::new();

    // Retrieve the list of CBZ to check.
    let books = env::args()
        .skip(1) // Skip the binary name.
        .map(|path| get_books(&client, Path::new(&path)))
        .collect::<Result<Vec<_>>>()
        .context("failed to collect paths")?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    // Check each book.
    for book in books {
        match book.check(&client) {
            Ok(errors) => {
                // No error? Great!
                if errors.is_empty() {
                    termio::print_ok(book.file_name());
                } else {
                    // Report every error detected.
                    termio::print_err(book.file_name());
                    println!("Checked against {}", book.ref_url().as_str());
                    for err in errors {
                        println!("==> {err}");
                    }
                }
            },
            Err(err) => {
                // Failed to even check the book, inform the user.
                termio::print_err(&format!(
                    "failed to check {}: {err:?}",
                    book.file_name(),
                ));
            },
        }
        println!();
    }

    Ok(())
}

/// Get every CBZ file under `path`.
///
/// If `path` is a CBZ instead of a directory, it's returned directly.
fn get_books(
    client: &bedetheque::Client,
    path: &Path,
) -> Result<Vec<cbz::Book>> {
    // Case 1. `path` is a file.
    if !path.is_dir() {
        return Ok(match cbz::Book::new(client, path) {
            Ok(cbz) => vec![cbz],
            Err(err) => {
                skip_file(path, &err);
                vec![]
            },
        });
    }
    // Case 2. `path` is a directory.
    fs::read_dir(path)
        .with_context(|| format!("failed to read dir {}", path.display()))?
        .filter_map(|res| {
            match res {
                Ok(entry) => {
                    match cbz::Book::new(client, &entry.path()) {
                        Ok(cbz) => Some(Ok(cbz)),
                        Err(err) => {
                            skip_file(&entry.path(), &err);
                            None // Skip this file.
                        },
                    }
                },
                Err(err) => {
                    Some(Err(err).with_context(|| {
                        format!("cannot access entry under {}", path.display())
                    }))
                },
            }
        })
        .collect::<Result<Vec<_>>>()
}

fn skip_file(path: &Path, err: &anyhow::Error) {
    termio::print_warn(&format!("skip {}: {err}", path.display()));
}
