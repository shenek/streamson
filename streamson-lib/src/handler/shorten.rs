//! Handler which shortens matched data
//! it can be used e.g. shorten long strings
//! `"some long text"` -> `"some lon..."`
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let handler = Arc::new(Mutex::new(handler::Shorten::new(3, r#"..""#.to_string())));
//! let matcher = matcher::Simple::new(r#"{"elements"}[]{"description"}"#).unwrap();
//!
//! let mut convert = strategy::Convert::new();
//!
//! // Set the matcher for convert strategy
//! convert.add_matcher(Box::new(matcher), vec![handler]);
//!
//! for input in vec![
//!     br#"{"elements": [{"description": "too long string"}, {"#.to_vec(),
//!     br#""description": "other long string"}]}"#.to_vec(),
//! ] {
//!     for converted_data in convert.process(&input).unwrap() {
//!         println!("{:?} (len {})", converted_data, converted_data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, streamer::ParsedKind, Path};

/// Handler which shortens the matched data
///
#[derive(Debug)]
pub struct Shorten {
    /// max length of original data
    max_length: usize,

    /// How shortened data are supposed to be terminated
    /// Note that the new data length = max_length + terminator.length
    terminator: String,
}

impl Shorten {
    /// Creates a new handler which shortens matched data
    pub fn new(max_length: usize, terminator: String) -> Self {
        Self {
            max_length,
            terminator,
        }
    }
}

impl Handler for Shorten {
    fn handle(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        data: Option<&[u8]>,
        _kind: ParsedKind,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let data = data.unwrap(); // buffering is required -> data should not be None
        if data.len() <= self.max_length {
            return Ok(Some(data.to_vec()));
        }

        let result: Vec<u8> = data[..self.max_length + 1]
            .iter()
            .cloned()
            .chain(self.terminator.as_bytes().iter().cloned())
            .collect();

        Ok(Some(result))
    }

    fn buffering_required(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Shorten;
    use crate::{matcher::Simple, strategy::Convert};
    use std::sync::{Arc, Mutex};

    #[test]
    fn shorten_handler() {
        let mut convert = Convert::new();
        let shorten_handler = Arc::new(Mutex::new(Shorten::new(10, "..\"".to_string())));
        let matcher = Simple::new(r#"[]{"description"}"#).unwrap();

        convert.add_matcher(Box::new(matcher), vec![shorten_handler.clone()]);
        let output = convert
            .process(br#"[{"description": "too long description"}, {"description": "short"}]"#)
            .unwrap();

        let output: Vec<u8> = output.into_iter().flatten().collect();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            r#"[{"description": "too long d.."}, {"description": "short"}]"#
        );
    }
}
