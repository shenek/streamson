//! Handler which pefroms regex conversion on mathed data
//!
//! # Example
//! ```
//! use streamson_lib::{matcher, strategy, handler};
//! use std::sync::{Arc, Mutex};
//! use regex;
//!
//! let converter =
//! Arc::new(Mutex::new(handler::Regex::new().add_regex(regex::Regex::new("User").unwrap(), "user".to_string(), 0)));
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
//!         println!("{:?} (len {})", converted_data, converted_data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, path::Path, streamer::Token};
use std::str;

/// Regex to match and string to convert to
type Replacement = (regex::Regex, String, usize);

/// Converts data using regex
#[derive(Debug, Default)]
pub struct Regex {
    /// All replacements which will be triggered
    replacements: Vec<Replacement>,
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
        for (regex, into, limit) in &self.replacements {
            let str_to_replace: &str = &into;
            output = regex.replacen(&output, *limit, str_to_replace).to_string();
        }

        // Clear the buffer so it can be reused later
        self.buffer.clear();

        Ok(Some(output.as_bytes().to_vec()))
    }

    fn is_converter(&self) -> bool {
        true
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
    /// * `regex` - regex which will be used
    /// * `into` - regex replacement
    pub fn add_regex(mut self, regex: regex::Regex, into: String, limit: usize) -> Self {
        self.replacements.push((regex, into, limit));
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{handler, matcher::Simple, strategy::Convert};
    use regex::Regex;
    use std::sync::{Arc, Mutex};

    #[test]
    fn regex_converter() {
        let mut convert = Convert::new();

        let regex_converter = handler::Regex::new().add_regex(
            Regex::new("[Uu]ser([0-9]+)").unwrap(),
            "user$1".to_string(),
            1,
        );

        let matcher = Simple::new(r#"[]{"name"}"#).unwrap();
        convert.add_matcher(Box::new(matcher), Arc::new(Mutex::new(regex_converter)));

        let output = convert
            .process(br#"[{"name": "User1 User1"}, {"name": "user2"}]"#)
            .unwrap();

        let output: Vec<u8> = output.into_iter().flatten().collect();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            r#"[{"name": "user1 User1"}, {"name": "user2"}]"#
        );
    }
}