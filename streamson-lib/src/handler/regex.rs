//! Handler which pefroms regex conversion on mathed data
//!
//! # Example
//! ```
//! use streamson_lib::{matcher, strategy::{self, Strategy}, handler};
//! use std::sync::{Arc, Mutex};
//! use regex;
//!
//! let converter =
//! Arc::new(Mutex::new(handler::Regex::new().add_regex("s/bad/good/g".to_string())));
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
//!
//! let mut convert = strategy::Convert::new();
//!
//! // Set the matcher for convert strategy
//! convert.add_matcher(Box::new(matcher), converter);
//!
//! for input in vec![
//!     br#"{"users": [{"password": "1234", "name": "User1"}, {"#.to_vec(),
//!     br#""password": "0000", "name": "user2}]}"#.to_vec(),
//! ] {
//!     for converted_data in convert.process(&input).unwrap() {
//!         println!("{:?}", converted_data);
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, path::Path, streamer::Token};
use std::{any::Any, str, str::FromStr};

/// Converts data using regex
#[derive(Default)]
pub struct Regex {
    /// All replacements (sed string)
    replacements: Vec<String>,
    /// Buffer to collect input
    buffer: Vec<u8>,
}

impl Handler for Regex {
    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self.buffer.extend(data);
        Ok(None)
    }

    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let mut output: String = str::from_utf8(&self.buffer)
            .map_err(|e| error::Handler::new(e.to_string()))?
            .to_string();
        output = sedregex::find_and_replace(&output, &self.replacements)
            .map_err(error::Handler::new)?
            .to_string();

        // Clear the buffer so it can be reused later
        self.buffer.clear();

        Ok(Some(output.as_bytes().to_vec()))
    }

    fn is_converter(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FromStr for Regex {
    type Err = error::Handler;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // Check format
        sedregex::ReplaceCommand::new(input).map_err(error::Handler::new)?;
        let mut new = Regex::new();
        new = new.add_regex(input.to_string());
        Ok(new)
    }
}

impl Regex {
    /// Creates a new regex converter
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds new regex conversion which will be applied
    ///
    /// # Arguments
    /// * `sedregex` - sed regex used to convert the data
    pub fn add_regex(mut self, sedregex: String) -> Self {
        self.replacements.push(sedregex);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        handler,
        matcher::Simple,
        strategy::{Convert, OutputConverter, Strategy},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn regex_converter() {
        let mut convert = Convert::new();

        let regex_converter =
            handler::Regex::new().add_regex("s/[Uu]ser([0-9]+)/user$1/".to_string());

        let matcher = Simple::new(r#"[]{"name"}"#).unwrap();
        convert.add_matcher(Box::new(matcher), Arc::new(Mutex::new(regex_converter)));

        let output: Vec<u8> = OutputConverter::new()
            .convert(
                &convert
                    .process(br#"[{"name": "User1 User1"}, {"name": "user2"}]"#)
                    .unwrap(),
            )
            .into_iter()
            .map(|e| e.1)
            .flatten()
            .collect();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            r#"[{"name": "user1 User1"}, {"name": "user2"}]"#
        );
    }
}
