//! Handler which stores indexes where matched data are kept.
//! The data should be within <start_idx, end_idx) range
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let indexer_handler = Arc::new(Mutex::new(handler::Indexer::new().set_use_path(true)));
//!
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
//!
//! let mut trigger = strategy::Trigger::new();
//!
//! // Set the matcher for trigger
//! trigger.add_matcher(Box::new(matcher), &[indexer_handler.clone()]);
//!
//! for input in vec![
//!     br#"{"users": [{"id": 1, "name": "first"}, {"#.to_vec(),
//!     br#""id": 2, "name": "second}]}"#.to_vec(),
//! ] {
//!     trigger.process(&input).unwrap();
//!     let mut guard = indexer_handler.lock().unwrap();
//!     while let Some((path, output)) = guard.pop() {
//!         // Do something with the data
//!         println!("{} ({:?})", path.unwrap(), output);
//!     }
//! }
//! ```

use super::Handler;
use crate::{
    error,
    path::Path,
    streamer::{Output, ParsedKind},
};
use std::collections::VecDeque;

/// Indexer handler responsible for storing index of the matches
#[derive(Debug)]
pub struct Indexer {
    /// Queue with stored indexes
    stored: VecDeque<(Option<String>, Output)>,

    /// Not to show path will spare some allocation
    use_path: bool,
}

impl Default for Indexer {
    fn default() -> Self {
        Self {
            stored: VecDeque::new(),
            use_path: false,
        }
    }
}

impl Handler for Indexer {
    fn handle(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _data: Option<&[u8]>,
        _kind: ParsedKind,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        Ok(None)
    }

    fn handle_idx(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        idx: Output,
    ) -> Result<(), error::Handler> {
        self.stored.push_back((
            if self.use_path {
                Some(path.to_string())
            } else {
                None
            },
            idx,
        ));
        Ok(())
    }

    fn use_path(&self) -> bool {
        self.use_path
    }

    fn buffering_required(&self) -> bool {
        // no need to buffer input
        // handler doesn't use matched data
        false
    }
}

impl Indexer {
    /// Creates a new handler which stores indexes within itself
    pub fn new() -> Self {
        Self::default()
    }
    ///
    /// Set whether to show path
    ///
    /// # Arguments
    /// * `use_path` - should path be store with data
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::Indexer::new().set_use_path(true);
    /// ```
    pub fn set_use_path(mut self, use_path: bool) -> Self {
        self.use_path = use_path;
        self
    }

    /// Pops the oldest value in the buffer
    ///
    /// # Returns
    /// * `None` - queue is empty
    /// * `Some((path, output))` - stored data remove from the queue and returned
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let mut indexer = handler::Indexer::new().set_use_path(true);
    /// while let Some((path, output)) = indexer.pop() {
    ///     // Do something with the data
    ///     println!("{} ({:?})", path.unwrap(), output);
    /// }
    ///
    ///
    /// ```
    pub fn pop(&mut self) -> Option<(Option<String>, Output)> {
        self.stored.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::Indexer;
    use crate::{
        handler::buffer::Buffer,
        matcher::Simple,
        strategy::Trigger,
        streamer::{Output, ParsedKind},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn indexer_handler() {
        let mut trigger = Trigger::new();

        let indexer_handler = Arc::new(Mutex::new(Indexer::new()));
        let buffer_handler = Arc::new(Mutex::new(Buffer::default()));
        let matcher_all = Simple::new(r#"{"elements"}"#).unwrap();
        let matcher_elements = Simple::new(r#"{"elements"}[]"#).unwrap();

        trigger.add_matcher(Box::new(matcher_all), &[indexer_handler.clone()]);
        trigger.add_matcher(
            Box::new(matcher_elements),
            &[indexer_handler.clone(), buffer_handler.clone()],
        );

        trigger.process(br#"{"elements": [1, 2, 3, 4]}"#).unwrap();

        // Test indexer handler
        let mut guard = indexer_handler.lock().unwrap();
        assert_eq!(guard.pop().unwrap(), (None, Output::Start(13)));
        assert_eq!(guard.pop().unwrap(), (None, Output::Start(14)));
        assert_eq!(
            guard.pop().unwrap(),
            (None, Output::End(15, ParsedKind::Num))
        );
        assert_eq!(guard.pop().unwrap(), (None, Output::Start(17)));
        assert_eq!(
            guard.pop().unwrap(),
            (None, Output::End(18, ParsedKind::Num))
        );
        assert_eq!(guard.pop().unwrap(), (None, Output::Start(20)));
        assert_eq!(
            guard.pop().unwrap(),
            (None, Output::End(21, ParsedKind::Num))
        );
        assert_eq!(guard.pop().unwrap(), (None, Output::Start(23)));
        assert_eq!(
            guard.pop().unwrap(),
            (None, Output::End(24, ParsedKind::Num))
        );
        assert_eq!(
            guard.pop().unwrap(),
            (None, Output::End(25, ParsedKind::Arr))
        );

        // Test whether buffer handler contains the right data
        let mut guard = buffer_handler.lock().unwrap();
        assert_eq!(guard.pop().unwrap(), (None, vec![b'1']));
        assert_eq!(guard.pop().unwrap(), (None, vec![b'2']));
        assert_eq!(guard.pop().unwrap(), (None, vec![b'3']));
        assert_eq!(guard.pop().unwrap(), (None, vec![b'4']));
    }
}
