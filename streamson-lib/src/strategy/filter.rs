//! The main logic of JSON filtering
//!
//! It uses matchers and filters matched parts
//! from output

use std::{collections::VecDeque, mem::swap};

use crate::{
    error,
    matcher::MatchMaker,
    path::Path,
    streamer::{Output, Streamer},
};

/// Processes data from input and remove matched parts (and keeps the json valid)
pub struct Filter {
    /// Buffer idx against total idx
    buffer_idx: usize,
    /// Buffer use for input buffering
    buffer: VecDeque<u8>,
    /// Responsible for data extraction
    streamer: Streamer,
    /// Matchers which will cause filtering
    matchers: Vec<Box<dyn MatchMaker>>,
    /// Path which is matched
    matched_path: Option<Path>,
    /// Level of last element which is not filtered
    last_output_level: usize,
    /// Index of last element which is not filtered
    last_output_idx: Option<usize>,
    /// discard on next not filtered start or end token
    delayed_discard: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            buffer_idx: 0,
            buffer: VecDeque::new(),
            matchers: vec![],
            streamer: Streamer::new(),
            matched_path: None,
            last_output_idx: None,
            delayed_discard: false,
            last_output_level: 0,
        }
    }
}

impl Filter {
    /// Create new filter
    ///
    /// It removes matched parts of the input
    pub fn new() -> Self {
        Self::default()
    }

    /// Split working buffer and return the removed part
    ///
    /// # Arguments
    /// * `idx` - total idx to split
    fn move_forward(&mut self, idx: usize) -> VecDeque<u8> {
        let mut splitted = self.buffer.split_off(idx - self.buffer_idx);

        // Swap to return cut part
        swap(&mut self.buffer, &mut splitted);

        self.buffer_idx = idx;

        splitted
    }

    /// Adds new matcher into filtering
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
    /// let mut filter = strategy::Filter::new();
    /// let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();
    /// filter.add_matcher(
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
    /// # Errors
    /// * Error is triggered when incorrect json is detected
    ///   Note that not all json errors are detected
    pub fn process(&mut self, input: &[u8]) -> Result<(Vec<u8>, bool), error::General> {
        // Feed the streamer
        self.streamer.feed(input);

        // Feed the input buffer
        self.buffer.extend(input);

        // initialize result
        let mut result = Vec::new();
        loop {
            match self.streamer.read()? {
                Output::Finished => {
                    if self.matched_path.is_none() {
                        if let Some(final_idx) = self.last_output_idx {
                            result.extend(self.move_forward(final_idx));
                        }
                    }
                    // we are done
                    return Ok((result, true));
                }
                Output::Pending => {
                    // need more data
                    return Ok((result, false));
                }
                Output::Start(idx) => {
                    // The path is not matched yet
                    if self.matched_path.is_none() {
                        // Discard first
                        if self.delayed_discard {
                            self.move_forward(idx);
                            self.delayed_discard = false;
                        }

                        let current_path = self.streamer.current_path().clone();

                        // check whether matches
                        if self.matchers.iter().any(|e| e.match_path(&current_path)) {
                            self.matched_path = Some(current_path);

                            // We can move idx forward (export last data which can be exported
                            let move_to_idx = if let Some(last_idx) = self.last_output_idx.take() {
                                last_idx
                            } else {
                                idx
                            };
                            result.extend(self.move_forward(move_to_idx));

                            // Special handling of first item in array / dict for output
                            if self.last_output_level < self.streamer.current_path().depth() {
                                self.delayed_discard = true;
                            }
                        } else {
                            self.last_output_idx = Some(idx + 1); // one element before
                            self.last_output_level = self.streamer.current_path().depth();
                        }
                    }
                }
                Output::End(idx) => {
                    if let Some(path) = self.matched_path.as_ref() {
                        if path == self.streamer.current_path() {
                            self.matched_path = None;

                            // move idx without storing it
                            if !self.delayed_discard {
                                self.move_forward(idx);
                            }
                        }
                    } else {
                        // Discard
                        if self.delayed_discard {
                            self.move_forward(idx - 1); // idx is on closing `]` or `}`
                            self.delayed_discard = false;
                        }

                        self.last_output_idx = Some(idx);
                        self.last_output_level = self.streamer.current_path().depth();
                    }
                }
                Output::Separator(idx) => {
                    if self.matched_path.is_none() {
                        if self.delayed_discard {
                            // special first child to filter case
                            self.move_forward(idx + 1); // rmeove with separator
                            self.delayed_discard = false;
                            self.last_output_idx = Some(idx + 1);
                        } else {
                            // just update output index to separator index
                            self.last_output_idx = Some(idx);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Filter;
    use crate::matcher::{Combinator, Simple};

    fn get_input() -> Vec<Vec<u8>> {
        vec![
            br#"{"users": [{"uid": 1}, {"uid": 2}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
                .iter()
                .map(|e| *e)
                .collect(),
        ]
    }

    #[test]
    fn single_matcher_no_match() {
        let input = get_input();

        let matcher = Simple::new(r#"{"no-existing"}[]{"uid"}"#).unwrap();
        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(filter.process(&input[0]).unwrap(), (input[0].clone(), true));
    }

    #[test]
    fn single_matcher_array_first() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[0]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{"users": [ {"uid": 2}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_array_last() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[2]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 2}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_array_middle() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[1]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_array_all() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{"users": [], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_object_first() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{ "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_object_last() {
        let input = get_input();
        let matcher = Simple::new(r#"{"void"}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 2}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}]}"#
        );
    }

    #[test]
    fn single_matcher_object_middle() {
        let input = get_input();
        let matcher = Simple::new(r#"{"groups"}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 2}, {"uid": 3}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_object_all() {
        let input = get_input();
        let matcher = Simple::new(r#"{}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher));

        assert_eq!(
            String::from_utf8(filter.process(&input[0]).unwrap().0).unwrap(),
            r#"{}"#
        );
    }

    #[test]
    fn combinator_slices() {
        let input = get_input();
        for i in 0..input.len() {
            let start_input = &input[0][0..i];
            let end_input = &input[0][i..];
            let matcher = Combinator::new(Simple::new(r#"{"users"}"#).unwrap())
                | Combinator::new(Simple::new(r#"{"void"}"#).unwrap());
            let mut filter = Filter::new();
            filter.add_matcher(Box::new(matcher));
            let mut result: Vec<u8> = Vec::new();

            result.extend(filter.process(&start_input).unwrap().0);
            result.extend(filter.process(&end_input).unwrap().0);
            assert_eq!(
                String::from_utf8(result).unwrap(),
                r#"{ "groups": [{"gid": 1}, {"gid": 2}]}"#
            )
        }
    }
}
