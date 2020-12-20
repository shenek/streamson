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
    streamer::{Output, Streamer},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct StackItem {
    /// Total index
    idx: usize,
    /// Idx to vec of matchers
    match_idx: usize,
    /// Is it required to buffer input data
    buffering_required: bool,
}

/// Item in matcher list
type MatcherItem = (Box<dyn MatchMaker>, Vec<Arc<Mutex<dyn Handler>>>);

/// Processes data from input and triggers handlers
pub struct Trigger {
    /// Input idx against total idx
    input_start: usize,
    /// Buffer index against total idx
    buffer_start: usize,
    /// Buffer which is used to store collected data
    buffer: Vec<u8>,
    /// Indicator whether data are collected
    collecting: bool,
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
            buffer_start: 0,
            buffer: vec![],
            collecting: false,
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
    ///     &[Arc::new(Mutex::new(handler))]
    /// );
    /// ```
    pub fn add_matcher(
        &mut self,
        matcher: Box<dyn MatchMaker>,
        handlers: &[Arc<Mutex<dyn Handler>>],
    ) {
        self.matchers.push((matcher, handlers.to_vec()));
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
                Output::Start(idx, kind) => {
                    // extend the input
                    let to = idx - self.input_start;
                    if self.collecting {
                        self.buffer.extend(&input[inner_idx..to]);
                    }
                    inner_idx = to;

                    let mut matched = vec![];
                    let path = self.streamer.current_path();

                    // try to check whether it matches
                    for (match_idx, (matcher, _)) in self.matchers.iter().enumerate() {
                        if matcher.match_path(path, kind) {
                            let mut buffering_required = false;
                            // handler starts
                            for handler in &self.matchers[match_idx].1 {
                                let mut guard = handler.lock().unwrap();
                                guard.handle_idx(path, match_idx, Output::Start(idx, kind))?;
                                if guard.buffering_required() {
                                    buffering_required = true
                                }
                            }

                            matched.push(StackItem {
                                idx,
                                match_idx,
                                buffering_required,
                            });

                            if !self.collecting & buffering_required {
                                // start the buffer
                                self.buffer_start = idx;
                                self.collecting = true;
                            }
                        }
                    }

                    self.matched_stack.push(matched);
                }
                Output::End(idx, kind) => {
                    let current_path = self.streamer.current_path();
                    let to = idx - self.input_start;
                    if self.collecting {
                        self.buffer.extend(&input[inner_idx..to]);
                    }
                    inner_idx = to;

                    let items = self.matched_stack.pop().unwrap();
                    for item in items {
                        // run handlers for the matches
                        for handler in &self.matchers[item.match_idx].1 {
                            let mut guard = handler.lock().unwrap();
                            guard.handle_idx(
                                current_path,
                                item.match_idx,
                                Output::End(idx, kind),
                            )?;
                            let buffering_required = guard.buffering_required();
                            guard.handle(
                                current_path,
                                item.match_idx,
                                if buffering_required {
                                    Some(
                                        &self.buffer
                                            [item.idx - self.buffer_start..idx - self.buffer_start],
                                    )
                                } else {
                                    None
                                },
                                kind,
                            )?;
                        }
                    }

                    // Clear the buffer if there is no need to keep the buffer
                    if self
                        .matched_stack
                        .iter()
                        .all(|items| items.iter().all(|item| !item.buffering_required))
                    {
                        self.collecting = false;
                        self.buffer.clear();
                    }
                }
                Output::Pending => {
                    self.input_start += input.len();
                    if self.collecting {
                        self.buffer.extend(&input[inner_idx..]);
                    }
                    return Ok(());
                }
                Output::Separator(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Trigger;
    use crate::{error, handler::Handler, matcher::Simple, path::Path, streamer::ParsedKind};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct TestHandler {
        paths: Vec<String>,
        data: Vec<Vec<u8>>,
    }

    impl Handler for TestHandler {
        fn handle(
            &mut self,
            path: &Path,
            _matcher_idx: usize,
            data: Option<&[u8]>,
            _kind: ParsedKind,
        ) -> Result<Option<Vec<u8>>, error::Handler> {
            self.paths.push(path.to_string());
            self.data.push(data.unwrap().to_vec());
            Ok(None)
        }
    }

    #[test]
    fn basic() {
        let mut trigger = Trigger::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
        trigger.add_matcher(Box::new(matcher), &[handler.clone()]);
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

    #[test]
    fn basic_pending() {
        let mut trigger = Trigger::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
        trigger.add_matcher(Box::new(matcher), &[handler.clone()]);

        trigger.process(br#"{"elem"#).unwrap();
        trigger.process(br#"ents": [1, 2, 3, 4]}"#).unwrap();

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
}
