//! Handler which replaces output by fixed data
//! it can be used a.g. to clear the sensitive data
//! e.g. `"secred_password"` -> `"***"
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let handler = Arc::new(Mutex::new(handler::Replace::new(br#"***"#.to_vec())));
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"password"}"#).unwrap();
//!
//! let mut convert = strategy::Convert::new();
//!
//! // Set the matcher for convert strategy
//! convert.add_matcher(Box::new(matcher), vec![handler]);
//!
//! for input in vec![
//!     br#"{"users": [{"password": "1234", "name": "first"}, {"#.to_vec(),
//!     br#""password": "0000", "name": "second}]}"#.to_vec(),
//! ] {
//!     for converted_data in convert.process(&input).unwrap() {
//!         println!("{:?} (len {})", converted_data, converted_data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, Path};

/// Replace handler which converts matched data to fixed output
#[derive(Debug)]
pub struct Replace {
    /// Data which will be returned instead of matched data
    new_data: Vec<u8>,
}

impl Replace {
    /// Creates a new handler which replaces matched data by fixed output
    pub fn new(new_data: Vec<u8>) -> Self {
        Self { new_data }
    }
}

impl Handler for Replace {
    fn handle(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _data: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        Ok(Some(self.new_data.clone()))
    }
}
