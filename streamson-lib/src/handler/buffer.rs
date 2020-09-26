//! Handler which buffers output which can be manually extracted
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy};
//! use std::sync::{Arc, Mutex};
//!
//! let buffer_handler = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));
//!
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
//!
//! let mut trigger = strategy::Trigger::new();
//!
//! // Set the matcher for trigger strategy
//! trigger.add_matcher(Box::new(matcher), &[buffer_handler.clone()]);
//!
//! for input in vec![
//!     br#"{"users": [{"id": 1, "name": "first"}, {"#.to_vec(),
//!     br#""id": 2, "name": "second}]}"#.to_vec(),
//! ] {
//!     trigger.process(&input).unwrap();
//!     let mut guard = buffer_handler.lock().unwrap();
//!     while let Some((path, data)) = guard.pop() {
//!         // Do something with the data
//!         println!("{} (len {})", path.unwrap(), data.len());
//!     }
//! }
//! ```

use super::Handler;
use crate::{error, path::Path};
use std::collections::VecDeque;

/// Buffer handler responsible for storing slitted JSONs into memory
#[derive(Debug)]
pub struct Buffer {
    /// Queue with stored jsons in (path, data) format
    stored: VecDeque<(Option<String>, Vec<u8>)>,

    /// Not to show path will spare some allocation
    use_path: bool,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            stored: VecDeque::new(),
            use_path: false,
        }
    }
}

impl Handler for Buffer {
    fn handle(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        data: Option<&[u8]>,
    ) -> Result<(), error::Handler> {
        // TODO we may limit the max VecDeque size and raise
        // an error when reached
        //
        let path_opt = if self.use_path {
            Some(path.to_string())
        } else {
            None
        };

        self.stored.push_back((path_opt, data.unwrap().to_vec()));
        Ok(())
    }

    fn use_path(&self) -> bool {
        self.use_path
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
    /// * `use_path` - should path be store with data
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::Buffer::new().set_use_path(true);
    /// ```
    pub fn set_use_path(mut self, use_path: bool) -> Self {
        self.use_path = use_path;
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
    /// let mut buffer = handler::buffer::Buffer::new().set_use_path(true);
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
