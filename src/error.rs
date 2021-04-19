use std::{
    collections::BTreeSet,
    fmt,
};

pub(crate) enum Error {
    Authors(String),
    Year(BTreeSet<u16>),
    Width { page: String, width: usize },
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
                    write!(f, "invalid years, expected {}", y[0])
                } else {
                    write!(f, "invalid years, expected one of {}", y.join(", "))
                }
            },
            Self::Width { page, width } => {
                write!(f, "unexpected width ({}) for {}", width, page)
            },
        }
    }
}
