#![crate_name = "streamson_extra_matchers"]

//! This library contains extra matchers for streamson-lib
//!
//! It contains extra dependencies so it moved to a separate library
//!

#[cfg(feature = "with_regex")]
pub mod regex_matcher;

#[cfg(feature = "with_regex")]
pub use regex_matcher::Regex;
