//! Handler which stores indexes where matched data are kept.
//! The data should be within <start_idx, end_idx) range
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, Collector};
//! use std::sync::{Arc, Mutex};
//!
//! let indexer_handler = Arc::new(Mutex::new(handler::Indexer::new().set_show_path(true)));
//!
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
//!
//! let mut collector = Collector::new();
//!
//! // Set the matcher for collector
//! collector.add_matcher(Box::new(matcher), &[indexer_handler.clone()]);
//!
//! for input in vec![
//!     br#"{"users": [{"id": 1, "name": "first"}, {"#.to_vec(),
//!     br#""id": 2, "name": "second}]}"#.to_vec(),
//! ] {
//!     collector.process(&input).unwrap();
//!     let mut guard = indexer_handler.lock().unwrap();
//!     while let Some((path, output)) = guard.pop() {
//!         // Do something with the data
//!         println!("{} ({:?})", path.unwrap(), output);
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, path::Path, streamer::Output};
use std::collections::VecDeque;

/// Indexer handler responsible for storing index of the matches
#[derive(Debug)]
pub struct Indexer {
    /// Queue with stored indexes
    stored: VecDeque<(Option<String>, Output)>,

    /// Not to show path will spare some allocation
    show_path: bool,
}

impl Default for Indexer {
    fn default() -> Self {
        Self {
            stored: VecDeque::new(),
            show_path: false,
        }
    }
}

impl Handler for Indexer {
    fn handle(&mut self, _path: &Path, _data: &[u8]) -> Result<(), error::Handler> {
        Ok(())
    }

    fn handle_idx(&mut self, path: &Path, idx: Output) -> Result<(), error::Handler> {
        self.stored.push_back((
            if self.show_path {
                Some(path.to_string())
            } else {
                None
            },
            idx,
        ));
        Ok(())
    }

    fn show_path(&self) -> bool {
        self.show_path
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
    /// * `show_path` - should path be store with data
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::Indexer::new().set_show_path(true);
    /// ```
    pub fn set_show_path(mut self, show_path: bool) -> Self {
        self.show_path = show_path;
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
    /// let mut indexer = handler::Indexer::new().set_show_path(true);
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
