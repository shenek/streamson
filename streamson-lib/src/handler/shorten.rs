//! Handler which shortens matched data
//! it can be used e.g. shorten long strings
//! `"some long text"` -> `"some lon..."`
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy::{self, Strategy}};
//! use std::sync::{Arc, Mutex};
//!
//! let handler = Arc::new(Mutex::new(handler::Shorten::new(3, r#"..""#.to_string())));
//! let matcher = matcher::Simple::new(r#"{"elements"}[]{"description"}"#).unwrap();
//!
//! let mut convert = strategy::Convert::new();
//!
//! // Set the matcher for convert strategy
//! convert.add_matcher(Box::new(matcher), handler);
//!
//! for input in vec![
//!     br#"{"elements": [{"description": "too long string"}, {"#.to_vec(),
//!     br#""description": "other long string"}]}"#.to_vec(),
//! ] {
//!     for converted_data in convert.process(&input).unwrap() {
//!         println!("{:?}", converted_data);
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, path::Path, streamer::Token};
use std::{any::Any, str::FromStr};

/// Handler which shortens the matched data
///
#[derive(Debug)]
pub struct Shorten {
    /// max length of original data
    max_length: usize,

    /// how many letters were already used
    used: usize,

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
            used: 0,
        }
    }
}

impl FromStr for Shorten {
    type Err = error::Handler;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<_> = input.split(',').collect();
        if splitted.len() == 2 {
            Ok(Self::new(
                splitted[0].parse().map_err(error::Handler::new)?,
                splitted[1].to_string(),
            ))
        } else {
            Err(error::Handler::new("Failed to parse"))
        }
    }
}

impl Handler for Shorten {
    fn start(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        // clear the used so it can start matching again
        self.used = 0;

        Ok(None)
    }

    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        // all letters were used
        if self.used >= self.max_length {
            return Ok(None);
        }

        // entire data can be consumed
        if data.len() <= (self.max_length - self.used) {
            self.used += data.len();
            return Ok(Some(data.to_vec()));
        }

        // terminates now
        let result: Vec<u8> = data[..(self.max_length - self.used + 1)]
            .iter()
            .cloned()
            .chain(self.terminator.as_bytes().iter().cloned())
            .collect();

        self.used = self.max_length;

        Ok(Some(result))
    }

    fn is_converter(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Shorten;
    use crate::{
        matcher::Simple,
        strategy::{Convert, OutputConverter, Strategy},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn shorten_handler() {
        let mut convert = Convert::new();
        let shorten_handler = Arc::new(Mutex::new(Shorten::new(10, "..\"".to_string())));
        let matcher = Simple::new(r#"[]{"description"}"#).unwrap();

        convert.add_matcher(Box::new(matcher), shorten_handler.clone());
        let output: Vec<u8> = OutputConverter::new()
            .convert(
                &convert
                    .process(
                        br#"[{"description": "too long description"}, {"description": "short"}]"#,
                    )
                    .unwrap(),
            )
            .into_iter()
            .map(|e| e.1)
            .flatten()
            .collect();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            r#"[{"description": "too long d.."}, {"description": "short"}]"#
        );
    }

    #[test]
    fn shorten_handler_parts() {
        let mut convert = Convert::new();
        let shorten_handler = Arc::new(Mutex::new(Shorten::new(10, "..\"".to_string())));
        let matcher = Simple::new(r#"[]{"description"}"#).unwrap();

        convert.add_matcher(Box::new(matcher), shorten_handler.clone());
        let mut output = convert.process(br#"[{"description": "t"#).unwrap();

        output.extend(convert.process(br#"oo long d"#).unwrap());

        output.extend(
            convert
                .process(br#"escription"}, {"description": "short"}]"#)
                .unwrap(),
        );

        let output: Vec<u8> = OutputConverter::new()
            .convert(&output)
            .into_iter()
            .map(|e| e.1)
            .flatten()
            .collect();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            r#"[{"description": "too long d.."}, {"description": "short"}]"#
        );
    }
}
