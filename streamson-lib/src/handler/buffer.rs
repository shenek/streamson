//! Handler which buffers output which can be manually extracted
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, Collector};
//! use std::sync::{Arc, Mutex};
//!
//! let buffer_handler = Arc::new(Mutex::new(handler::Buffer::new().set_show_path(true)));
//!
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
//!
//! let mut collector = Collector::new();
//!
//! // Set the matcher for collector
//! collector = collector.add_matcher(Box::new(matcher), &[buffer_handler.clone()]);
//!
//! for input in vec![
//!     br#"{"users": [{"id": 1, "name": "first"}, {"#.to_vec(),
//!     br#""id": 2, "name": "second}]}"#.to_vec(),
//! ] {
//!     collector.process(&input).unwrap();
//!     let mut guard = buffer_handler.lock().unwrap();
//!     while let Some((path, data)) = guard.pop() {
//!         // Do something with the data
//!         println!("{} (len {})", path.unwrap(), data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::error;
use std::collections::VecDeque;

/// Buffer handler responsible for storing slitted JSONs into memory
#[derive(Debug)]
pub struct Buffer {
    /// Queue with stored jsons in (path, data) format
    stored: VecDeque<(Option<String>, Vec<u8>)>,

    /// Not to show path will spare some allocation
    show_path: bool,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            stored: VecDeque::new(),
            show_path: false,
        }
    }
}

impl Handler for Buffer {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Handler> {
        // TODO we may limit the max VecDeque size and raise
        // an error when reached
        //
        let path_opt = if self.show_path {
            Some(path.to_string())
        } else {
            None
        };

        self.stored.push_back((path_opt, data.to_vec()));
        Ok(())
    }

    fn show_path(&self) -> bool {
        self.show_path
    }
}

impl Buffer {
    /// Creates a new handler which stores output within itself
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
    /// let file = handler::Buffer::new().set_show_path(true);
    /// ```
    pub fn set_show_path(mut self, show_path: bool) -> Self {
        self.show_path = show_path;
        self
    }

    /// Pops the oldest value in the buffer
    ///
    /// # Returns
    /// * `None` - queue is empty
    /// * `Some((path, data))` - stored data remove from the queue and returned
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let mut buffer = handler::buffer::Buffer::new().set_show_path(true);
    /// while let Some((path, data)) = buffer.pop() {
    ///     // Do something with the data
    ///     println!("{} (len {})", path.unwrap(), data.len());
    /// }
    ///
    ///
    /// ```
    pub fn pop(&mut self) -> Option<(Option<String>, Vec<u8>)> {
        self.stored.pop_front()
    }
}
