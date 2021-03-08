//! The main logic of trigger JSON processing
//!
//! It uses matchers, handlers and path extraction,
//! to call a handler on the matched part of data
//!
//! Note that it doesn't change the json while processing,
//! which makes it the fastest strategy in streamson-lib.

use crate::{
    error,
    handler::Handler,
    matcher::MatchMaker,
    streamer::{Streamer, Token},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct StackItem {
    /// Total index
    idx: usize,
    /// Idx to vec of matchers
    match_idx: usize,
}

/// Item in matcher list
type MatcherItem = (Box<dyn MatchMaker>, Arc<Mutex<dyn Handler>>);

/// Processes data from input and triggers handlers
pub struct Trigger {
    /// Input idx against total idx
    input_start: usize,
    /// Path matchers and handlers
    matchers: Vec<MatcherItem>,
    /// Responsible for data extraction
    streamer: Streamer,
    /// Matched stack
    matched_stack: Vec<Vec<StackItem>>,
}

impl Default for Trigger {
    fn default() -> Self {
        Self {
            input_start: 0,
            matchers: vec![],
            streamer: Streamer::new(),
            matched_stack: vec![],
        }
    }
}

impl Trigger {
    /// Creates a new `Trigger`
    ///
    /// It collects matched data and triggers handlers when entire
    /// data are read.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a mathcher and a handler to `Trigger`
    ///
    /// # Arguments
    /// * `matcher` - matcher which matches the path
    /// * `handlers` - list of handlers to be triggers when path matches
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{strategy, matcher, handler};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut trigger = strategy::Trigger::new();
    /// let handler = handler::PrintLn::new();
    /// let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();
    /// trigger.add_matcher(
    ///     Box::new(matcher),
    ///     Arc::new(Mutex::new(handler))
    /// );
    /// ```
    pub fn add_matcher(&mut self, matcher: Box<dyn MatchMaker>, handler: Arc<Mutex<dyn Handler>>) {
        self.matchers.push((matcher, handler));
    }

    fn feed(&mut self, data: &[u8]) -> Result<(), error::Handler> {
        for matched_items in &self.matched_stack {
            for matched_item in matched_items {
                let mut guard = self.matchers[matched_item.match_idx].1.lock().unwrap();
                guard.feed(data, matched_item.match_idx)?;
            }
        }
        Ok(())
    }

    /// Processes input data
    ///
    /// # Arguments
    /// * `input` - input data
    ///
    /// # Returns
    /// * `Ok(()) processing passed, more data might be needed
    /// * `Err(_)` - error occured during processing
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::strategy;
    ///
    /// let mut trigger = strategy::Trigger::new();
    /// trigger.process(br#"{}"#);
    /// ```
    ///
    /// # Errors
    ///
    /// If parsing logic finds that JSON is not valid,
    /// it returns `error::General`.
    ///
    /// Note that streamson assumes that its input is a valid
    /// JSONs and if not, it still might be processed without an error.
    /// This is caused because streamson does not validate JSON.
    pub fn process(&mut self, input: &[u8]) -> Result<(), error::General> {
        self.streamer.feed(input);
        let mut inner_idx = 0;
        loop {
            match self.streamer.read()? {
                Token::Start(idx, kind) => {
                    // trigger handler for matched
                    let to = idx - self.input_start;
                    self.feed(&input[inner_idx..to])?;
                    inner_idx = to;

                    let mut matched = vec![];
                    let path = self.streamer.current_path();

                    // try to check whether it matches
                    for (match_idx, (matcher, _)) in self.matchers.iter().enumerate() {
                        if matcher.match_path(path, kind) {
                            // handler starts
                            let mut guard = self.matchers[match_idx].1.lock().unwrap();
                            guard.start(path, match_idx, Token::Start(idx, kind))?;
                            matched.push(StackItem { idx, match_idx });
                        }
                    }

                    self.matched_stack.push(matched);
                }
                Token::End(idx, kind) => {
                    let to = idx - self.input_start;
                    self.feed(&input[inner_idx..to])?;
                    inner_idx = to;

                    let current_path = self.streamer.current_path();
                    let items = self.matched_stack.pop().unwrap();
                    for item in items {
                        // run handlers for the matches
                        let mut guard = self.matchers[item.match_idx].1.lock().unwrap();
                        guard.end(current_path, item.match_idx, Token::End(idx, kind))?;
                    }
                }
                Token::Pending => {
                    self.input_start += input.len();
                    self.feed(&input[inner_idx..])?;
                    return Ok(());
                }
                Token::Separator(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Trigger;
    use crate::{
        error,
        handler::Handler,
        matcher::Simple,
        path::Path,
        streamer::Token,
        test::{Single, Splitter, Window},
    };
    use rstest::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct TestHandler {
        paths: Vec<String>,
        data: Vec<Vec<u8>>,
        current: Vec<u8>,
    }

    impl Handler for TestHandler {
        fn start(
            &mut self,
            path: &Path,
            _matcher_idx: usize,
            _kind: Token,
        ) -> Result<Option<Vec<u8>>, error::Handler> {
            self.paths.push(path.to_string());
            Ok(None)
        }
        fn feed(
            &mut self,
            data: &[u8],
            _matcher_idx: usize,
        ) -> Result<Option<Vec<u8>>, error::Handler> {
            self.current.extend(data.to_vec());
            Ok(None)
        }
        fn end(
            &mut self,
            _path: &Path,
            _matcher_idx: usize,
            _kind: Token,
        ) -> Result<Option<Vec<u8>>, error::Handler> {
            self.data.push(self.current.clone());
            self.current.clear();
            Ok(None)
        }
    }

    #[test]
    fn basic() {
        let mut trigger = Trigger::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
        trigger.add_matcher(Box::new(matcher), handler.clone());
        trigger.process(br#"{"elements": [1, 2, 3, 4]}"#).unwrap();

        let guard = handler.lock().unwrap();
        assert_eq!(guard.paths[0], r#"{"elements"}[0]"#);
        assert_eq!(guard.data[0], br#"1"#.to_vec());

        assert_eq!(guard.paths[1], r#"{"elements"}[1]"#);
        assert_eq!(guard.data[1], br#"2"#.to_vec());

        assert_eq!(guard.paths[2], r#"{"elements"}[2]"#);
        assert_eq!(guard.data[2], br#"3"#.to_vec());

        assert_eq!(guard.paths[3], r#"{"elements"}[3]"#);
        assert_eq!(guard.data[3], br#"4"#.to_vec());
    }

    #[rstest(
        splitter,
        case::single(Box::new(Single::new())),
        case::window1(Box::new(Window::new(1))),
        case::window5(Box::new(Window::new(5))),
        case::window100(Box::new(Window::new(100)))
    )]
    fn splitted(splitter: Box<dyn Splitter>) {
        for parts in splitter.split(br#"{"elements": [1, 2, 3, 4]}"#.to_vec()) {
            let mut trigger = Trigger::new();
            let handler = Arc::new(Mutex::new(TestHandler::default()));
            let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
            trigger.add_matcher(Box::new(matcher), handler.clone());

            for part in parts {
                trigger.process(&part).unwrap();
            }

            let guard = handler.lock().unwrap();
            assert_eq!(guard.paths.len(), 4);
            //assert_eq!(guard.data.len(), 4);
            assert_eq!(guard.paths[0], r#"{"elements"}[0]"#);
            assert_eq!(guard.data[0], br#"1"#.to_vec());

            assert_eq!(guard.paths[1], r#"{"elements"}[1]"#);
            assert_eq!(guard.data[1], br#"2"#.to_vec());

            assert_eq!(guard.paths[2], r#"{"elements"}[2]"#);
            assert_eq!(guard.data[2], br#"3"#.to_vec());

            assert_eq!(guard.paths[3], r#"{"elements"}[3]"#);
            assert_eq!(guard.data[3], br#"4"#.to_vec());
        }
    }
}
