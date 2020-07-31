//! The main logic of JSON processing
//!
//! It puts together matchers, handlers and path extraction.

use crate::{
    error,
    handler::Handler,
    matcher::MatchMaker,
    path::{Emitter, Output},
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
    /// Emits path from data
    emitter: Emitter,
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
            emitter: Emitter::new(),
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
    /// let collector = Collector::new().add_matcher(
    ///     Box::new(matcher),
    ///     &[Arc::new(Mutex::new(handler))]
    /// );
    /// ```
    pub fn add_matcher(
        mut self,
        matcher: Box<dyn MatchMaker>,
        handlers: &[Arc<Mutex<dyn Handler>>],
    ) -> Self {
        self.matchers.push((matcher, handlers.to_vec()));
        self
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
        self.emitter.feed(input);
        let mut inner_idx = 0;
        loop {
            match self.emitter.read()? {
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
                    let path = self.emitter.current_path();

                    // try to check whether it matches
                    for (match_idx, (matcher, _)) in self.matchers.iter().enumerate() {
                        if matcher.match_path(path) {
                            matched.push(StackItem { idx, match_idx });
                            if !self.collecting {
                                // start the buffer
                                self.buffer_start = idx;
                                self.collecting = true;
                            }
                        }
                    }

                    self.matched_stack.push(matched);
                }
                Output::End(idx) => {
                    let current_path = self.emitter.current_path();
                    let to = idx - self.input_start;
                    if self.collecting {
                        self.buffer.extend(&input[inner_idx..to]);
                    }
                    inner_idx = to;

                    let items = self.matched_stack.pop().unwrap();
                    for item in items {
                        // matches
                        for handler in &self.matchers[item.match_idx].1 {
                            handler.lock().unwrap().handle(
                                current_path,
                                &self.buffer[item.idx - self.buffer_start..idx - self.buffer_start],
                            )?;
                        }
                    }

                    // Clear the buffer if there is no need to keep the buffer
                    if self.matched_stack.iter().all(|items| items.is_empty()) {
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
        fn handle(&mut self, path: &Path, data: &[u8]) -> Result<(), error::Handler> {
            self.paths.push(path.to_string());
            self.data.push(data.to_vec());
            Ok(())
        }
    }

    #[test]
    fn basic() {
        let mut collector = Collector::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#).unwrap();
        collector = collector.add_matcher(Box::new(matcher), &[handler.clone()]);

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
        collector = collector.add_matcher(Box::new(matcher), &[handler.clone()]);

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
