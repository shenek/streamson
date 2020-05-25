//! Module containing errors

use std::{error::Error, fmt, io, str::Utf8Error};

/// Matcher related errors
#[derive(Debug, PartialEq, Clone)]
pub enum Matcher {
    Parse(String),
}

impl Error for Matcher {}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Parse(input) => write!(f, "Failed to parse matcher'{}", input),
        }
    }
}

/// Handler related errors
#[derive(Debug, PartialEq, Clone)]
pub struct Handler {
    reason: String,
}

impl Handler {
    pub fn new<T>(reason: T) -> Self
    where
        T: ToString,
    {
        Self {
            reason: reason.to_string(),
        }
    }
}

impl Error for Handler {}

impl fmt::Display for Handler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Handler failed - {}", self.reason)
    }
}

/// Incorrect input error
#[derive(Debug, PartialEq, Clone)]
pub struct IncorrectInput {
    chr: char,
    idx: usize,
}

impl IncorrectInput {
    pub fn new(chr: char, idx: usize) -> Self {
        Self { chr, idx }
    }
}

impl Error for IncorrectInput {}

impl fmt::Display for IncorrectInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Incorrect input (character '{}' on idx {})",
            self.chr, self.idx
        )
    }
}
/// Handler related errors
#[derive(Debug)]
pub enum General {
    Handler(Handler),
    Matcher(Matcher),
    Utf8Error(Utf8Error),
    IncorrectInput(IncorrectInput),
    IOError(io::Error),
}

impl Error for General {}
impl fmt::Display for General {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Handler(err) => err.fmt(f),
            Self::Matcher(err) => err.fmt(f),
            Self::Utf8Error(err) => err.fmt(f),
            Self::IncorrectInput(err) => err.fmt(f),
            Self::IOError(err) => err.fmt(f),
        }
    }
}

impl From<Handler> for General {
    fn from(handler: Handler) -> Self {
        Self::Handler(handler)
    }
}

impl From<Matcher> for General {
    fn from(matcher: Matcher) -> Self {
        Self::Matcher(matcher)
    }
}

impl From<Utf8Error> for General {
    fn from(utf8: Utf8Error) -> Self {
        Self::Utf8Error(utf8)
    }
}

impl From<IncorrectInput> for General {
    fn from(incorrect_input: IncorrectInput) -> Self {
        Self::IncorrectInput(incorrect_input)
    }
}

impl From<io::Error> for General {
    fn from(io_error: io::Error) -> Self {
        Self::IOError(io_error)
    }
}
