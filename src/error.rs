use std::{
    collections::BTreeSet,
    fmt,
};

pub(crate) enum Error {
    Authors(String),
    Year(BTreeSet<u16>),
    Width,
    Date,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authors(authors) => {
                write!(f, "invalid authors, expected ({})", authors)
            },
            Self::Year(y) => {
                let y = y.iter().map(ToString::to_string).collect::<Vec<_>>();
                if y.len() == 1 {
                    write!(f, "invalid year, expected {}", y[0])
                } else {
                    write!(f, "invalid year, expected one of {}", y.join(", "))
                }
            },
            Self::Width => {
                write!(f, "some images have unexpected width")
            },
            Self::Date => {
                write!(f, "some images have an unexpected last modified date")
            },
        }
    }
}
