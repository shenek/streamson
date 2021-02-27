#![crate_name = "streamson_lib"]

//! This library is able to process large JSON data.
//!
//! It can have various matchers which matches the path (see [matcher](matcher/index.html))
//!
//! And various handlers to do something with found data (see [handlers](handler/index.html))
//!
//! # Examples
//! ```
//! use streamson_lib::{handler::{self, Handler}, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let stdout_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
//!
//! let handler = handler::Group::new()
//!     .add_handler(Arc::new(Mutex::new(handler::File::new("out.txt").unwrap())))
//!     .add_handler(stdout_handler.clone());
//!
//! let first_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
//! let second_matcher = matcher::Simple::new(r#"{"groups"}[]"#).unwrap();
//!
//! let mut trigger = strategy::Trigger::new();
//!
//! // exports users to stdout and out.txt
//! trigger.add_matcher(
//!     Box::new(first_matcher),
//!     Arc::new(Mutex::new(handler.clone())),
//! );
//!
//! // groups are going to be expoted only to stdout
//! trigger.add_matcher(
//!     Box::new(second_matcher),
//!     stdout_handler,
//! );
//!
//! for input in vec![
//!     br#"{"users": [1,2]"#.to_vec(),
//!     br#", "groups": [3, 4]}"#.to_vec(),
//! ] {
//!     trigger.process(&input).unwrap();
//! }
//! ```
//!
//! ```
//! use streamson_lib::{handler::{self, Handler}, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let file_handler = Arc::new(
//!     Mutex::new(handler::File::new("out.txt").unwrap())
//! );
//! let stdout_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
//! let handler = handler::Group::new()
//!     .add_handler(stdout_handler)
//!     .add_handler(file_handler);
//!
//! let first_matcher = matcher::Depth::new(1, Some(2));
//! let second_matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
//! let matcher = matcher::Combinator::new(first_matcher) |
//!     matcher::Combinator::new(second_matcher);
//!
//! let mut trigger = strategy::Trigger::new();
//!
//! // Paths with depths 1, 2 are exported to out.txt
//! trigger.add_matcher(
//!     Box::new(matcher),
//!     Arc::new(Mutex::new(handler)),
//! );
//!
//! for input in vec![
//!     br#"{"users": [1,2]"#.to_vec(),
//!     br#", "groups": [3, 4]}"#.to_vec(),
//! ] {
//!     trigger.process(&input).unwrap();
//! }
//! ```
//!
//! ```
//! use streamson_lib::{handler::{self, Handler}, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let file_handler = Arc::new(
//!     Mutex::new(handler::File::new("out.txt").unwrap())
//! );
//! let stdout_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
//! let handler = handler::Group::new()
//!     .add_handler(stdout_handler)
//!     .add_handler(file_handler);
//!
//! let matcher = matcher::Depth::new(1, Some(2));
//!
//! let mut trigger = strategy::Trigger::new();
//!
//! // Paths with depths 1, 2 are exported to out.txt
//! trigger.add_matcher(
//!     Box::new(matcher),
//!     Arc::new(Mutex::new(handler)),
//! );
//!
//! for input in vec![
//!     br#"{"users": [1,2]"#.to_vec(),
//!     br#", "groups": [3, 4]}"#.to_vec(),
//! ] {
//!     trigger.process(&input).unwrap();
//! }
//! ```

pub mod error;
pub mod handler;
pub mod matcher;
pub mod path;
pub mod strategy;
pub mod streamer;

pub use handler::Handler;
pub use path::Path;
pub use streamer::{Streamer, Token};

#[cfg(doctest)]
mod test_readme {
    macro_rules! external_doc_test {
        ($x:expr) => {
            #[doc = $x]
            extern {}
        };
    }
    external_doc_test!(include_str!("../README.md"));
}
