//! Handler which unstringifies matched data
//! it can be used e.g. shorten long strings
//! `"{\"aa\": {\"bb\":2}, \"cc\": \"dd\"}"` -> `{"aa": {"bb": 2}, "cc": "dd"}`
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let handler = Arc::new(Mutex::new(handler::Unstringify::new()));
//! let matcher = matcher::Simple::new(r#"{"stringified_strings"}[]"#).unwrap();
//!
//! let mut convert = strategy::Convert::new();
//!
//! // Set the matcher for convert strategy
//! convert.add_matcher(Box::new(matcher), vec![handler]);
//!
//! for input in vec![
//!     br#"{"stringified_strings": ["\"string\"", "{}", "[]"]}"#.to_vec(),
//! ] {
//!     for converted_data in convert.process(&input).unwrap() {
//!         println!("{:?} (len {})", converted_data, converted_data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, Path};

/// Handler which unstringifies the matched data
///
#[derive(Debug, Default)]
pub struct Unstringify;

impl Unstringify {
    /// Creates a new handler which unstringifies matched data
    pub fn new() -> Self {
        Self
    }
}

impl Handler for Unstringify {
    fn handle(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        data: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let data = data.unwrap(); // buffering is required -> data should not be None
        if data[0] != b'"' || data[data.len() - 1] != b'"' {
            return Err(error::Handler::new(
                "Unstringified data is supposed to be a string.",
            ));
        }

        let mut result: Vec<u8> = vec![];
        let mut escaped = false;
        for byte in data.iter().take(data.len() - 1).skip(1) {
            if escaped {
                result.push(*byte);
                escaped = false;
            } else {
                match *byte {
                    b'\\' => escaped = true,
                    b'"' => return Err(error::Handler::new("Wrong unstringify format")),
                    byte => result.push(byte),
                }
            }
        }
        if escaped {
            return Err(error::Handler::new("Wrong unstringify format"));
        }

        Ok(Some(result))
    }

    fn buffering_required(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Unstringify;
    use crate::{matcher::Simple, strategy::Convert};
    use std::sync::{Arc, Mutex};

    #[test]
    fn unstringify_handler_ok() {
        let mut convert = Convert::new();
        let shorten_handler = Arc::new(Mutex::new(Unstringify::new()));
        let matcher = Simple::new(r#"[]{"stringified"}[]"#).unwrap();

        convert.add_matcher(Box::new(matcher), vec![shorten_handler.clone()]);
        let mut output = convert
            .process(br#"[{"stringified": ["true", "false", "null", "11", "\"\""]},"#)
            .unwrap();

        output.extend(
            convert
                .process(br#" {"stringified": ["\"inner\"", "[]", "{}", "{\"key\": \"value\"}"]}]"#)
                .unwrap(),
        );

        let output: Vec<u8> = output.into_iter().flatten().collect();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            r#"[{"stringified": [true, false, null, 11, ""]}, {"stringified": ["inner", [], {}, {"key": "value"}]}]"#
        );
    }
}
