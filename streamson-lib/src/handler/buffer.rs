//! Handler which buffers output which can be manually extracted
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, Collector};
//! use std::sync::{Arc, Mutex};
//!
//! let buffer_handler = Arc::new(Mutex::new(handler::Buffer::new()));
//!
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#);
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
//!         println!("{} (len {})", path, data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::error;
use bytes::Bytes;
use std::collections::VecDeque;

/// Buffer handler responsible for storing slitted JSONs into memory
#[derive(Debug, Default)]
pub struct Buffer {
    /// Queue with stored jsons in (path, data) format
    stored: VecDeque<(String, Bytes)>,
}

impl Handler for Buffer {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic> {
        // TODO we may limit the max VecDeque size and raise
        // an error when reached

        self.stored
            .push_back((path.to_string(), Bytes::from(data.to_vec())));
        Ok(())
    }
}

impl Buffer {
    /// Creates a new handler which stores output within itself
    pub fn new() -> Self {
        Self::default()
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
    /// let mut buffer = handler::buffer::Buffer::new();
    /// while let Some((path, data)) = buffer.pop() {
    ///     // Do something with the data
    ///     println!("{} (len {})", path, data.len());
    /// }
    ///
    ///
    /// ```
    pub fn pop(&mut self) -> Option<(String, Bytes)> {
        self.stored.pop_front()
    }
}
