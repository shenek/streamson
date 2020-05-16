//! Module containing errors

use std::{error::Error, fmt};

/// Generic Error
///
/// Currently the only error kind is used
#[derive(Debug, PartialEq, Clone)]
pub struct Generic;

impl Error for Generic {}

impl fmt::Display for Generic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericError")
    }
}
