//! Handler which unstringifies matched data
//! it can be used e.g. shorten long strings
//! `"{\"aa\": {\"bb\":2}, \"cc\": \"dd\"}"` -> `{"aa": {"bb": 2}, "cc": "dd"}`
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy::{self, Strategy}};
//! use std::sync::{Arc, Mutex};
//!
//! let handler = Arc::new(Mutex::new(handler::Unstringify::new()));
//! let matcher = matcher::Simple::new(r#"{"stringified_strings"}[]"#).unwrap();
//!
//! let mut convert = strategy::Convert::new();
//!
//! // Set the matcher for convert strategy
//! convert.add_matcher(Box::new(matcher), handler);
//!
//! for input in vec![
//!     br#"{"stringified_strings": ["\"string\"", "{}", "[]"]}"#.to_vec(),
//! ] {
//!     for converted_data in convert.process(&input).unwrap() {
//!         println!("{:?}", converted_data);
//!     }
//! }
//! ```

use super::Handler;
use crate::{
    error,
    streamer::{ParsedKind, Token},
    Path,
};

#[derive(Debug)]
pub enum State {
    Initial,
    Escaping,
    Processing,
    Terminated,
}

impl Default for State {
    fn default() -> Self {
        Self::Initial
    }
}

fn _processing_error() -> error::Handler {
    error::Handler::new("Wrong unstringify format")
}

/// Handler which unstringifies the matched data
///
#[derive(Debug, Default)]
pub struct Unstringify {
    state: State,
}

impl Unstringify {
    /// Creates a new handler which unstringifies matched data
    pub fn new() -> Self {
        Default::default()
    }
}

impl Handler for Unstringify {
    fn start(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self.state = State::Initial;
        if let Token::Start(_, kind) = token {
            if !matches!(kind, ParsedKind::Str) {
                return Err(error::Handler::new(
                    "Unstringified data is supposed to be a string.",
                ));
            }
            Ok(None)
        } else {
            unreachable!();
        }
    }

    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let mut result: Vec<u8> = vec![];
        for byte in data.iter() {
            match self.state {
                State::Initial => {
                    // skip first character
                    self.state = State::Processing;
                    continue;
                }
                State::Processing => {
                    match *byte {
                        b'\\' => self.state = State::Escaping,
                        b'"' => {
                            // terminate handler matching
                            self.state = State::Terminated;
                            break;
                        }
                        byte => result.push(byte),
                    }
                }
                State::Escaping => {
                    // Just append next byte
                    result.push(*byte);
                    self.state = State::Processing;
                }
                State::Terminated => return Err(_processing_error()),
            }
        }

        Ok(Some(result))
    }

    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if !matches!(self.state, State::Terminated) {
            return Err(error::Handler::new("String does not ended"));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::Unstringify;
    use crate::{
        matcher::Simple,
        strategy::{Convert, OutputConverter, Strategy},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn unstringify_handler_ok() {
        let mut convert = Convert::new();
        let shorten_handler = Arc::new(Mutex::new(Unstringify::new()));
        let matcher = Simple::new(r#"[]{"stringified"}[]"#).unwrap();

        convert.add_matcher(Box::new(matcher), shorten_handler.clone());
        let mut output = convert
            .process(br#"[{"stringified": ["true", "false", "null", "11", "\"\""]},"#)
            .unwrap();

        output.extend(
            convert
                .process(br#" {"stringified": ["\"inner\"", "[]", "{}", "{\"key\": \"value\"}"]}]"#)
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
            r#"[{"stringified": [true, false, null, 11, ""]}, {"stringified": ["inner", [], {}, {"key": "value"}]}]"#
        );
    }
}
