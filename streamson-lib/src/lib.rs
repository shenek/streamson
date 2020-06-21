#![crate_name = "streamson_lib"]

//! This library is able to split JSON into separate parts.
//!
//! It can have various matchers which matches the path (see [matcher](matcher/index.html))
//!
//! And various handlers to do something with found data (see [handlers](handler/index.html))
//!
//! # Examples
//! ```
//! use streamson_lib::{handler::{self, Handler}, matcher, Collector};
//! use std::sync::{Arc, Mutex};
//!
//! let file_handler = Arc::new(
//!     Mutex::new(handler::File::new("/tmp/out.txt").unwrap())
//! );
//! let stdout_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
//!
//! let first_matcher = matcher::Simple::new(r#"{"users"}[]"#);
//! let second_matcher = matcher::Simple::new(r#"{"groups"}[]"#);
//!
//! let mut collector = Collector::new();
//!
//! // exports users to stdout and /tmp/out.txt
//! collector = collector.add_matcher(
//!     Box::new(first_matcher),
//!     &[stdout_handler.clone(), file_handler],
//! );
//!
//! // groups are going to be expoted only to stdout
//! collector = collector.add_matcher(
//!     Box::new(second_matcher),
//!     &[stdout_handler],
//! );
//!
//! for input in vec![
//!     br#"{"users": [1,2]"#.to_vec(),
//!     br#", "groups": [3, 4]}"#.to_vec(),
//! ] {
//!     collector.process(&input).unwrap();
//! }
//! ```
//!
//! ```
//! use streamson_lib::{handler::{self, Handler}, matcher, Collector};
//! use std::sync::{Arc, Mutex};
//!
//! let file_handler = Arc::new(
//!     Mutex::new(handler::File::new("/tmp/out.txt").unwrap())
//! );
//! let stdout_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
//!
//! let first_matcher = matcher::Depth::new(1, Some(2));
//! let second_matcher = matcher::Simple::new(r#"{"users"}[]"#);
//! let matcher = matcher::Combinator::new(first_matcher) |
//!     matcher::Combinator::new(second_matcher);
//!
//! let mut collector = Collector::new();
//!
//! // Paths with depths 1, 2 are exported to tmp.txt
//! collector = collector.add_matcher(
//!     Box::new(matcher),
//!     &[stdout_handler.clone(), file_handler],
//! );
//!
//! for input in vec![
//!     br#"{"users": [1,2]"#.to_vec(),
//!     br#", "groups": [3, 4]}"#.to_vec(),
//! ] {
//!     collector.process(&input).unwrap();
//! }
//! ```
//!
//! ```
//! use streamson_lib::{handler::{self, Handler}, matcher, Collector};
//! use std::sync::{Arc, Mutex};
//!
//! let file_handler = Arc::new(
//!     Mutex::new(handler::File::new("/tmp/out.txt").unwrap())
//! );
//! let stdout_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
//!
//! let matcher = matcher::Depth::new(1, Some(2));
//!
//! let mut collector = Collector::new();
//!
//! // Paths with depths 1, 2 are exported to tmp.txt
//! collector = collector.add_matcher(
//!     Box::new(matcher),
//!     &[stdout_handler.clone(), file_handler],
//! );
//!
//! for input in vec![
//!     br#"{"users": [1,2]"#.to_vec(),
//!     br#", "groups": [3, 4]}"#.to_vec(),
//! ] {
//!     collector.process(&input).unwrap();
//! }
//! ```

pub mod collector;
pub mod error;
pub mod handler;
pub mod matcher;
pub mod path;

pub use collector::Collector;
pub use handler::Handler;
pub use path::{Emitter, Output};
