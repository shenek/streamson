use crate::{
    error,
    handler::Handler,
    matcher::MatchMaker,
    path::{Emitter, Output},
};
use bytes::BytesMut;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct StackItem {
    /// Total index
    idx: usize,
    /// Path which was matched
    path: String,
    /// Idx to vec of matchers
    match_idx: usize,
}

type MatcherItem = (Box<dyn MatchMaker>, Vec<Arc<Mutex<dyn Handler>>>);

#[derive(Default)]
pub struct Collector {
    /// Input idx against total idx
    input_start: usize,
    /// Buffer index against total idx
    buffer_start: usize,
    /// Buffer which is used to store collected data
    buffer: Option<BytesMut>,
    /// Path matchers and handlers
    matchers: Vec<MatcherItem>,
    /// Emits path from data
    emitter: Emitter,
    /// Path stack
    path_stack: Vec<StackItem>,
}

impl Collector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_matcher(
        mut self,
        matcher: Box<dyn MatchMaker>,
        handlers: &[Arc<Mutex<dyn Handler>>],
    ) -> Self {
        self.matchers.push((matcher, handlers.to_vec()));
        self
    }

    pub fn process(&mut self, input: &[u8]) -> Result<bool, error::Generic> {
        self.emitter.feed(input);
        let mut inner_idx = 0;
        loop {
            match self.emitter.read()? {
                Output::Finished => {
                    return Ok(true);
                }
                Output::Start(idx, path) => {
                    let to = idx - self.input_start;
                    if let Some(stored) = self.buffer.as_mut() {
                        stored.extend(&input[inner_idx..to]);
                    }
                    inner_idx = to;

                    // try to check whether it matches
                    for (match_idx, (matcher, _)) in self.matchers.iter().enumerate() {
                        if matcher.match_path(&path) {
                            self.path_stack.push(StackItem {
                                idx,
                                match_idx,
                                path: path.clone(),
                            });
                            if self.buffer.is_none() {
                                // start the buffer
                                self.buffer_start = idx;
                                self.buffer = Some(BytesMut::new());
                            }
                        }
                    }
                }
                Output::End(idx, path) => {
                    let to = idx - self.input_start;
                    if let Some(stored) = self.buffer.as_mut() {
                        stored.extend(&input[inner_idx..to]);
                    }
                    inner_idx = to;

                    if let Some(item) = self.path_stack.pop() {
                        if item.path == path {
                            // matches
                            for handler in &self.matchers[item.match_idx].1 {
                                handler.lock().unwrap().handle(
                                    &path,
                                    &self.buffer.as_ref().unwrap()
                                        [item.idx - self.buffer_start..idx - self.buffer_start],
                                )?;
                            }
                            if self.path_stack.is_empty() {
                                self.buffer = None; // clear the buffer
                            }
                            continue;
                        } else {
                            // put back the previous item
                            self.path_stack.push(item);
                        }
                    }
                }
                Output::Pending => {
                    self.input_start += input.len();
                    if let Some(stored) = self.buffer.as_mut() {
                        stored.extend(&input[inner_idx..]);
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
    use crate::{error, handler::Handler, matcher::Simple};
    use bytes::Bytes;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct TestHandler {
        paths: Vec<String>,
        data: Vec<Bytes>,
    }

    impl Handler for TestHandler {
        fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic> {
            self.paths.push(path.to_string());
            self.data.push(Bytes::from(data.to_vec()));
            Ok(())
        }
    }

    #[test]
    fn basic() {
        let mut collector = Collector::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#);
        collector = collector.add_matcher(Box::new(matcher), &[handler.clone()]);

        assert!(
            collector.process(br#"{"elements": [1, 2, 3, 4]}"#).unwrap(),
            true
        );
        let guard = handler.lock().unwrap();
        assert_eq!(guard.paths[0], r#"{"elements"}[0]"#);
        assert_eq!(guard.data[0], Bytes::from(br#"1"#.to_vec()));

        assert_eq!(guard.paths[1], r#"{"elements"}[1]"#);
        assert_eq!(guard.data[1], Bytes::from(br#"2"#.to_vec()));

        assert_eq!(guard.paths[2], r#"{"elements"}[2]"#);
        assert_eq!(guard.data[2], Bytes::from(br#"3"#.to_vec()));

        assert_eq!(guard.paths[3], r#"{"elements"}[3]"#);
        assert_eq!(guard.data[3], Bytes::from(br#"4"#.to_vec()));
    }

    #[test]
    fn basic_pending() {
        let mut collector = Collector::new();
        let handler = Arc::new(Mutex::new(TestHandler::default()));
        let matcher = Simple::new(r#"{"elements"}[]"#);
        collector = collector.add_matcher(Box::new(matcher), &[handler.clone()]);

        assert_eq!(collector.process(br#"{"elem"#).unwrap(), false);
        assert_eq!(collector.process(br#"ents": [1, 2, 3, 4]}"#).unwrap(), true);

        let guard = handler.lock().unwrap();
        assert_eq!(guard.paths[0], r#"{"elements"}[0]"#);
        assert_eq!(guard.data[0], Bytes::from(br#"1"#.to_vec()));

        assert_eq!(guard.paths[1], r#"{"elements"}[1]"#);
        assert_eq!(guard.data[1], Bytes::from(br#"2"#.to_vec()));

        assert_eq!(guard.paths[2], r#"{"elements"}[2]"#);
        assert_eq!(guard.data[2], Bytes::from(br#"3"#.to_vec()));

        assert_eq!(guard.paths[3], r#"{"elements"}[3]"#);
        assert_eq!(guard.data[3], Bytes::from(br#"4"#.to_vec()));
    }
}
