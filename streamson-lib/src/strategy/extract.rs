//! The main logic of JSON extracting
//!
//! It uses matchers to extract a parts of JSON.
//! Nested matches have no meaning here

use crate::{
    error,
    handler::Handler,
    matcher::MatchMaker,
    path::Path,
    streamer::{Streamer, Token},
};
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
pub enum Output {
    Start(Option<Path>),
    Data(Vec<u8>),
    End,
}

type MatcherItem = (Box<dyn MatchMaker>, Option<Arc<Mutex<dyn Handler>>>);

pub struct Extract {
    /// Export path as well
    export_path: bool,
    /// Input idx against total idx
    input_start: usize,
    /// What is currently matched - path and indexes to matchers
    matches: Option<(Path, Vec<usize>)>,
    /// Path matchers
    matchers: Vec<MatcherItem>,
    /// Creates to token stream
    streamer: Streamer,
}

impl Default for Extract {
    fn default() -> Self {
        Self {
            export_path: false,
            input_start: 0,
            matches: None,
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

    /// Sets whether path should be exported with data
    ///
    /// if path is not exported extraction can be a bit faster
    pub fn set_export_path(mut self, export: bool) -> Self {
        self.export_path = export;
        self
    }

    /// Adds new matcher for data extraction
    ///
    /// # Arguments
    /// * `matcher` - matcher which matches the path
    /// * `handler` - optinal handler to be used to process data
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
    ///     None,
    /// );
    /// ```
    pub fn add_matcher(
        &mut self,
        matcher: Box<dyn MatchMaker>,
        handler: Option<Arc<Mutex<dyn Handler>>>,
    ) {
        self.matchers.push((matcher, handler));
    }

    /// Processes input data
    ///
    /// # Returns
    /// * `Ok(Vec<(Some(r#"{"users"}[0]"#), Vec<u8>)>)` vector containing path and data
    /// * `Ok(Vec<(None, Vec<u8>)>)` vector data only
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
    pub fn process(&mut self, input: &[u8]) -> Result<Vec<Output>, error::General> {
        self.streamer.feed(input);

        let mut input_idx = 0;

        let mut result = vec![];
        loop {
            match self.streamer.read()? {
                Token::Start(idx, kind) => {
                    if self.matches.is_none() {
                        let path = self.streamer.current_path();

                        // try to check whether it matches
                        let mut matched_indexes = vec![];
                        for (matcher_idx, (matcher, _handler)) in self.matchers.iter().enumerate() {
                            if matcher.match_path(path, kind) {
                                matched_indexes.push(matcher_idx);
                            }
                        }
                        if !matched_indexes.is_empty() {
                            // New match appears here
                            input_idx = idx - self.input_start;
                            for matcher_idx in &matched_indexes {
                                if let Some(handler) = self.matchers[*matcher_idx].1.as_ref() {
                                    let mut guard = handler.lock().unwrap();
                                    // triger handlers start
                                    guard.start(path, *matcher_idx, Token::Start(idx, kind))?;
                                }
                            }
                            self.matches = Some((path.clone(), matched_indexes));

                            // Set output
                            result.push(Output::Start(if self.export_path {
                                Some(path.clone())
                            } else {
                                None
                            }));
                        }
                    }
                }
                Token::Pending => {
                    if let Some((_, matched_indexes)) = self.matches.as_ref() {
                        for matcher_idx in matched_indexes {
                            if let Some(handler) = self.matchers[*matcher_idx].1.as_ref() {
                                let mut guard = handler.lock().unwrap();
                                // feed handlers
                                guard.feed(&input[input_idx..], *matcher_idx)?;
                            }
                        }
                        result.push(Output::Data(input[input_idx..].to_vec()));
                    }
                    self.input_start += input.len();
                    return Ok(result);
                }
                Token::End(idx, kind) => {
                    if let Some((path, matched_indexes)) = self.matches.as_ref() {
                        // Put the data to results
                        if path == self.streamer.current_path() {
                            let old_idx = input_idx;
                            input_idx = idx - self.input_start;
                            result.push(Output::Data(input[old_idx..input_idx].to_vec()));
                            result.push(Output::End);
                            // Feed and end handlers
                            for matcher_idx in matched_indexes {
                                if let Some(handler) = self.matchers[*matcher_idx].1.as_ref() {
                                    let mut guard = handler.lock().unwrap();
                                    // feed handlers
                                    guard.feed(&input[old_idx..input_idx], *matcher_idx)?;
                                    guard.end(&path, *matcher_idx, Token::End(idx, kind))?;
                                }
                            }
                            self.matches = None;
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
    use super::{Extract, Output};
    use crate::{
        handler::Buffer,
        matcher::Simple,
        path::Path,
        test::{Single, Splitter, Window},
    };
    use rstest::*;
    use std::{
        convert::TryFrom,
        sync::{Arc, Mutex},
    };

    fn get_input() -> Vec<u8> {
        br#"{"users": [{"name": "fred"}, {"name": "bob"}], "groups": [{"name": "admins"}]}"#
            .to_vec()
    }

    #[test]
    fn flat() {
        // without path
        let input = get_input();
        let matcher = Simple::new(r#"{}[]{"name"}"#).unwrap();

        let mut extract = Extract::new();
        extract.add_matcher(Box::new(matcher.clone()), None);

        let output = extract.process(&input).unwrap();
        assert_eq!(output.len(), 9);
        assert_eq!(output[0], Output::Start(None));
        assert_eq!(output[1], Output::Data(br#""fred""#.to_vec()));
        assert_eq!(output[2], Output::End);
        assert_eq!(output[3], Output::Start(None));
        assert_eq!(output[4], Output::Data(br#""bob""#.to_vec()));
        assert_eq!(output[5], Output::End);
        assert_eq!(output[6], Output::Start(None));
        assert_eq!(output[7], Output::Data(br#""admins""#.to_vec()));
        assert_eq!(output[8], Output::End);

        // with path
        let input = get_input();
        let mut extract = Extract::new().set_export_path(true);
        extract.add_matcher(Box::new(matcher), None);
        let output = extract.process(&input).unwrap();
        assert_eq!(output.len(), 9);
        assert_eq!(
            output[0],
            Output::Start(Some(Path::try_from(r#"{"users"}[0]{"name"}"#).unwrap()))
        );
        assert_eq!(output[1], Output::Data(br#""fred""#.to_vec()));
        assert_eq!(output[2], Output::End);
        assert_eq!(
            output[3],
            Output::Start(Some(Path::try_from(r#"{"users"}[1]{"name"}"#).unwrap()))
        );
        assert_eq!(output[4], Output::Data(br#""bob""#.to_vec()));
        assert_eq!(output[5], Output::End);
        assert_eq!(
            output[6],
            Output::Start(Some(Path::try_from(r#"{"groups"}[0]{"name"}"#).unwrap()))
        );
        assert_eq!(output[7], Output::Data(br#""admins""#.to_vec()));
        assert_eq!(output[8], Output::End);
    }

    #[test]
    fn nested() {
        let input = get_input();
        let matcher = Simple::new(r#"{}[1]"#).unwrap();

        let mut extract = Extract::new();
        extract.add_matcher(Box::new(matcher), None);

        let output = extract.process(&input).unwrap();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0], Output::Start(None));
        assert_eq!(output[1], Output::Data(br#"{"name": "bob"}"#.to_vec()));
        assert_eq!(output[2], Output::End);
    }

    #[test]
    fn pending() {
        let input = get_input();
        let input1 = &input[0..37];
        let input2 = &input[37..];

        let matcher = Simple::new(r#"{}[1]"#).unwrap();

        let mut extract = Extract::new();
        extract.add_matcher(Box::new(matcher), None);

        let output = extract.process(input1).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], Output::Start(None));
        assert_eq!(output[1], Output::Data(br#"{"name":"#.to_vec()));

        let output = extract.process(input2).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], Output::Data(br#" "bob"}"#.to_vec()));
        assert_eq!(output[1], Output::End);
    }

    #[test]
    fn pending_handlers() {
        let input = get_input();
        let input1 = &input[0..37];
        let input2 = &input[37..];

        let matcher = Simple::new(r#"{}[1]"#).unwrap();
        let buffer_handler = Arc::new(Mutex::new(Buffer::new().set_max_buffer_size(Some(22))));

        let mut extract = Extract::new();
        extract.add_matcher(Box::new(matcher), Some(buffer_handler.clone()));

        let output = extract.process(input1).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], Output::Start(None));
        assert_eq!(output[1], Output::Data(br#"{"name":"#.to_vec()));

        let output = extract.process(input2).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], Output::Data(br#" "bob"}"#.to_vec()));
        assert_eq!(output[1], Output::End);

        assert_eq!(
            buffer_handler.lock().unwrap().pop().unwrap(),
            (None, br#"{"name": "bob"}"#.to_vec())
        );
    }

    #[rstest(
        splitter,
        case::single(Box::new(Single::new())),
        case::window1(Box::new(Window::new(1))),
        case::window5(Box::new(Window::new(5))),
        case::window100(Box::new(Window::new(100)))
    )]
    fn splitters(splitter: Box<dyn Splitter>) {
        for parts in splitter.split(get_input()) {
            let matcher = Simple::new(r#"{}[]{"name"}"#).unwrap();

            let mut extract = Extract::new();
            extract.add_matcher(Box::new(matcher.clone()), None);

            let mut res = vec![];
            for part in parts {
                let output = extract.process(&part).unwrap();
                for e in output {
                    match e {
                        Output::Data(data) => res.extend(data),
                        _ => {}
                    }
                }
            }
            assert_eq!(String::from_utf8(res).unwrap(), r#""fred""bob""admins""#)
        }
    }
}
