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
    byte: u8,
    idx: usize,
}

impl IncorrectInput {
    pub fn new(byte: u8, idx: usize) -> Self {
        Self { byte, idx }
    }
}

impl Error for IncorrectInput {}

impl fmt::Display for IncorrectInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Incorrect input (byte '{}' on idx {})",
            self.byte, self.idx
        )
    }
}

/// Input data stream ended but data were still expected
#[derive(Debug, PartialEq, Clone)]
pub struct InputTerminated {
    idx: usize,
}

impl InputTerminated {
    pub fn new(idx: usize) -> Self {
        Self { idx }
    }
}

impl Error for InputTerminated {}

impl fmt::Display for InputTerminated {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InputTerminated (idx '{}')", self.idx)
    }
}

/// Path related error
#[derive(Debug, PartialEq, Clone)]
pub struct Path {
    path: String,
}

impl Path {
    pub fn new<T>(path: T) -> Self
    where
        T: ToString,
    {
        Self {
            path: path.to_string(),
        }
    }
}

impl Error for Path {}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wrong path '{}'", self.path)
    }
}

/// General error
#[derive(Debug)]
pub enum General {
    Path(Path),
    Handler(Handler),
    Matcher(Matcher),
    Utf8Error(Utf8Error),
    IncorrectInput(IncorrectInput),
    InputTerminated(InputTerminated),
    IoError(io::Error),
}

impl Error for General {}
impl fmt::Display for General {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Path(err) => err.fmt(f),
            Self::Handler(err) => err.fmt(f),
            Self::Matcher(err) => err.fmt(f),
            Self::Utf8Error(err) => err.fmt(f),
            Self::IncorrectInput(err) => err.fmt(f),
            Self::InputTerminated(err) => err.fmt(f),
            Self::IoError(err) => err.fmt(f),
        }
    }
}

macro_rules! impl_into_general {
    ($tp:path, $inner: path) => {
        impl From<$tp> for General {
            fn from(entity: $tp) -> Self {
                $inner(entity)
            }
        }
    };
}

impl_into_general!(Path, Self::Path);
impl_into_general!(Handler, Self::Handler);
impl_into_general!(Matcher, Self::Matcher);
impl_into_general!(Utf8Error, Self::Utf8Error);
impl_into_general!(IncorrectInput, Self::IncorrectInput);
impl_into_general!(InputTerminated, Self::InputTerminated);
impl_into_general!(io::Error, Self::IoError);
