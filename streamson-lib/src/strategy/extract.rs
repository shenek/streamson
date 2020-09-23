//! The main logic of JSON extracting
//!
//! It uses matchers to extract a parts of JSON.
//! Nested matches have no meaning here

use crate::{
    error,
    matcher::MatchMaker,
    path::Path,
    streamer::{Output, Streamer},
};
use std::mem::swap;

pub struct Extract {
    /// Input idx against total idx
    input_start: usize,
    /// Buffer index against total idx
    buffer_start: usize,
    /// Buffer which is used to store collected data
    buffer: Vec<u8>,
    /// Path which is matched
    matched_path: Option<Path>,
    /// Path matchers
    matchers: Vec<Box<dyn MatchMaker>>,
    /// Creates to token stream
    streamer: Streamer,
}

impl Default for Extract {
    fn default() -> Self {
        Self {
            input_start: 0,
            buffer_start: 0,
            buffer: vec![],
            matched_path: None,
            matchers: vec![],
            streamer: Streamer::new(),
        }
    }
}

impl Extract {
    /// Creates a new `Extract`
    ///
    /// It exracts matched data parts (not nested)
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds new matcher for data extraction
    ///
    /// # Arguments
    /// * `matcher` - matcher which matches the path
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{strategy, matcher};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut extract = strategy::Extract::new();
    /// let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();
    /// let mut extract = strategy::Extract::new();
    /// extract.add_matcher(
    ///     Box::new(matcher),
    /// );
    /// ```
    pub fn add_matcher(&mut self, matcher: Box<dyn MatchMaker>) {
        self.matchers.push(matcher);
    }

    /// Processes input data
    ///
    /// # Returns
    /// * `Ok(_, true)` entire json processed
    /// * `Ok(_, false)` need more input data
    /// * `Err(_)` when input is not correct json
    ///
    /// # Example
    /// ```
    /// use streamson_lib::strategy;
    ///
    /// let mut extract = strategy::Extract::new();
    /// extract.process(br#"{}"#);
    /// ```
    ///
    /// # Errors
    /// * Error is triggered when incorrect json is detected
    ///   Note that not all json errors are detected
    pub fn process(&mut self, input: &[u8]) -> Result<(Vec<Vec<u8>>, bool), error::General> {
        self.streamer.feed(input);

        let mut input_idx = 0;

        let mut result = vec![];
        loop {
            match self.streamer.read()? {
                Output::Finished => {
                    return Ok((result, true));
                }
                Output::Start(idx) if self.matched_path.is_none() => {
                    let path = self.streamer.current_path();

                    // try to check whether it matches
                    for matcher in self.matchers.iter() {
                        if matcher.match_path(path) {
                            // start the buffer
                            self.buffer_start = idx;
                            self.matched_path = Some(path.clone());
                            input_idx = idx - self.input_start;
                        }
                    }
                }
                Output::Pending => {
                    self.input_start += input.len();
                    if self.matched_path.is_some() {
                        self.buffer.extend(&input[input_idx..]);
                    }
                    return Ok((result, false));
                }
                Output::End(idx) if self.matched_path.is_some() => {
                    if let Some(path) = self.matched_path.as_ref() {
                        if path == self.streamer.current_path() {
                            // extend buffer and update indexes
                            let to = idx - self.input_start;
                            self.buffer.extend(&input[input_idx..to]);
                            input_idx = to;

                            // Put the buffer to results
                            let mut buffer = vec![];
                            swap(&mut self.buffer, &mut buffer);
                            result.push(buffer);
                            self.matched_path = None
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Extract;
    use crate::matcher::Simple;

    fn get_input() -> Vec<Vec<u8>> {
        vec![
            br#"{"users": [{"name": "fred"}, {"name": "bob"}], "groups": [{"name": "admins"}]}"#
                .iter()
                .map(|e| *e)
                .collect(),
        ]
    }

    #[test]
    fn flat() {
        let input = get_input();
        let matcher = Simple::new(r#"{}[]{"name"}"#).unwrap();

        let mut extract = Extract::new();
        extract.add_matcher(Box::new(matcher));

        let (mut output, end) = extract.process(&input[0]).unwrap();
        assert_eq!(end, true);
        assert_eq!(output.len(), 3);
        assert_eq!(String::from_utf8(output.remove(0)).unwrap(), r#""fred""#);
        assert_eq!(String::from_utf8(output.remove(0)).unwrap(), r#""bob""#);
        assert_eq!(String::from_utf8(output.remove(0)).unwrap(), r#""admins""#);
    }

    #[test]
    fn nested() {
        let input = get_input();
        let matcher = Simple::new(r#"{}[1]"#).unwrap();

        let mut extract = Extract::new();
        extract.add_matcher(Box::new(matcher));

        let (mut output, end) = extract.process(&input[0]).unwrap();
        assert_eq!(end, true);
        assert_eq!(output.len(), 1);
        assert_eq!(
            String::from_utf8(output.remove(0)).unwrap(),
            r#"{"name": "bob"}"#
        );
    }

    // TODO write test were the input is splitted into two parts
}
