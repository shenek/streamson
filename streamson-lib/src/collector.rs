//! The main logic of JSON processing
//!
//! It puts together matchers, handlers and path extraction.

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
pub struct Collector {
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

impl Default for Collector {
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

impl Collector {
    /// Creates new collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a `Collector` with extended matcher and handlers
    ///
    /// # Arguments
    /// * `matcher` - matcher which matches the path
    /// * `handlers` - list of handlers to be triggers when path matches
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{Collector, matcher, handler};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut collector = Collector::new();
    /// let handler = handler::PrintLn::new();
    /// let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();
    /// let mut collector = Collector::new();
    /// collector.add_matcher(
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
    /// * `Ok(true)` - All data successfully processed
    /// * `Ok(false)` - Data were processed, but another input is required
    /// * `Err(_)` - error occured during processing
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::Collector;
    ///
    /// let mut collector = Collector::new();
    /// collector.process(br#"{}"#);
    /// ```
    ///
    /// # Errors
    ///
    /// If parsing logic finds that JSON is not valid,
    /// it returns `error::General`.
    ///
    /// Note that streamson assumes that its input is a valid
    /// JSONs and if not. It still might be splitted without an error.
    /// This is caused because streamson does not validate JSON.
    pub fn process(&mut self, input: &[u8]) -> Result<bool, error::General> {
        self.streamer.feed(input);
        let mut inner_idx = 0;
        loop {
            match self.streamer.read()? {
                Output::Finished => {
                    return Ok(true);
                }
                Output::Start(idx) => {
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
                        if matcher.match_path(path) {
                            let mut buffering_required = false;
                            // handler starts
                            for handler in &self.matchers[match_idx].1 {
                                let mut guard = handler.lock().unwrap();
                                guard.handle_idx(path, Output::Start(idx))?;
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
                Output::End(idx) => {
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
                            guard.handle_idx(current_path, Output::End(idx))?;
                            let buffering_required = guard.buffering_required();
                            guard.handle(
                                current_path,
                                if buffering_required {
                                    Some(
                                        &self.buffer
                                            [item.idx - self.buffer_start..idx - self.buffer_start],
                                    )
                                } else {
                                    None
                                },
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
                    return Ok(false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Collector;
    use crate::{error, handler::Handler, matcher::Simple, path::Path};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct TestHandler {
        paths: Vec<String>,
        data: Vec<Vec<u8>>,
    }

    impl Handler for TestHandler {
        fn handle(&mut self, path: &Path, data: Option<&[u8]>) -> Result<(), error::Handler> {
            self.paths.push(path.to_string());
            self.data.push(data.unwrap().to_vec());
            Ok(())
        }
    }

    #[test]
    fn basic() {
        let mut collector = Collector::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
        collector.add_matcher(Box::new(matcher), &[handler.clone()]);

        assert!(
            collector.process(br#"{"elements": [1, 2, 3, 4]}"#).unwrap(),
            true
        );
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
        let mut collector = Collector::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
        collector.add_matcher(Box::new(matcher), &[handler.clone()]);

        assert_eq!(collector.process(br#"{"elem"#).unwrap(), false);
        assert_eq!(collector.process(br#"ents": [1, 2, 3, 4]}"#).unwrap(), true);

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
