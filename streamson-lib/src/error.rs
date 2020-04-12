use std::{error::Error, fmt};

#[derive(Debug, PartialEq, Clone)]
pub struct GenericError;

impl Error for GenericError {}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericError")
    }
}
