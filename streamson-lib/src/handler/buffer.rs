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
//! trigger.add_matcher(Box::new(matcher), buffer_handler.clone());
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
use crate::{error, path::Path, streamer::Token};
use std::collections::VecDeque;

/// Buffer handler responsible for storing slitted JSONs into memory
#[derive(Debug)]
pub struct Buffer {
    /// For storing unterminated data
    buffer: Vec<u8>,

    /// Buffer idx to total index
    buffer_idx: usize,

    /// Indexes for the Path and size
    buffer_parts: Vec<usize>,

    /// Queue with stored jsons in (path, data) format
    results: VecDeque<(Option<String>, Vec<u8>)>,

    /// Not to show path will spare some allocation
    use_path: bool,

    /// Current buffer size (in bytes)
    current_buffer_size: usize,

    /// Max buffer size
    max_buffer_size: Option<usize>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            use_path: false,
            current_buffer_size: 0,
            max_buffer_size: None,
            buffer: vec![],
            buffer_idx: 0,
            buffer_parts: vec![],
            results: VecDeque::new(),
        }
    }
}

trait Buff: Handler {
    fn _start(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if let Token::Start(idx, _) = token {
            if self.buffer_parts().is_empty() {
                *self.buffer_idx() = idx;
            }
            let buffer_idx = *self.buffer_idx();
            self.buffer_parts().push(idx - buffer_idx);
            Ok(None)
        } else {
            Err(error::Handler::new("Invalid token"))
        }
    }

    fn _feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        // buffer is being used
        if !self.buffer_parts().is_empty() {
            // check whether buffer capacity hasn't been reached
            if let Some(limit) = *self.max_buffer_size() {
                if *self.current_buffer_size() + data.len() > limit {
                    return Err(error::Handler::new(format!(
                        "Max buffer size {} was reached",
                        limit
                    )));
                }
            }
            self.buffer().extend(data);
            *self.current_buffer_size() += data.len();
        }

        Ok(None)
    }

    fn _end(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        // Try to push buffer
        if let Some(idx) = self.buffer_parts().pop() {
            let data = self.buffer()[idx..].to_vec();
            self.store_result(path, data);
            if self.buffer_parts().is_empty() {
                self.buffer().clear();
            }
            Ok(None)
        } else {
            Err(error::Handler::new("Invalid token"))
        }
    }

    fn store_result(&mut self, path: &Path, data: Vec<u8>);
    fn buffer(&mut self) -> &mut Vec<u8>;
    fn buffer_parts(&mut self) -> &mut Vec<usize>;
    fn buffer_idx(&mut self) -> &mut usize;
    fn max_buffer_size(&mut self) -> &mut Option<usize>;
    fn current_buffer_size(&mut self) -> &mut usize;
    fn use_path(&mut self) -> &mut bool;
}

impl Handler for Buffer {
    fn start(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self._start(_path, _matcher_idx, token)
    }

    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self._feed(data, _matcher_idx)
    }

    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self._end(_path, _matcher_idx, token)
    }
}

impl Buff for Buffer {
    fn store_result(&mut self, path: &Path, data: Vec<u8>) {
        let use_path = *self.use_path();
        self.results.push_back((
            if use_path {
                Some(path.to_string())
            } else {
                None
            },
            data,
        ));
    }

    fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    fn buffer_parts(&mut self) -> &mut Vec<usize> {
        &mut self.buffer_parts
    }

    fn buffer_idx(&mut self) -> &mut usize {
        &mut self.buffer_idx
    }

    fn max_buffer_size(&mut self) -> &mut Option<usize> {
        &mut self.max_buffer_size
    }

    fn current_buffer_size(&mut self) -> &mut usize {
        &mut self.current_buffer_size
    }

    fn use_path(&mut self) -> &mut bool {
        &mut self.use_path
    }
}

impl Buffer {
    /// Creates a new handler which stores output within itself
    pub fn new() -> Self {
        Self::default()
    }

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
        let popped = self.results.pop_front();
        if let Some((_, buffer)) = popped.as_ref() {
            self.current_buffer_size -= buffer.len();
        }
        popped
    }

    /// Sets max buffer size
    ///
    /// # Arguments
    /// * `use_path` - should path be store with data
    pub fn set_max_buffer_size(mut self, max_size: Option<usize>) -> Self {
        self.max_buffer_size = max_size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Buffer;
    use crate::{matcher::Simple, strategy::Trigger};
    use std::sync::{Arc, Mutex};

    #[test]
    fn max_buffer_size_error() {
        let mut trigger = Trigger::new();
        let buffer_handler = Arc::new(Mutex::new(Buffer::new().set_max_buffer_size(Some(22))));
        let matcher = Simple::new(r#"[]{"description"}"#).unwrap();

        trigger.add_matcher(Box::new(matcher), buffer_handler.clone());

        // Fits into the buffer
        assert!(trigger.process(br#"[{"description": "short"}, "#).is_ok());
        // Doesn't fit into the buffer
        assert!(trigger
            .process(br#"{"description": "too long description"}]"#)
            .is_err());
    }

    #[test]
    fn max_buffer_size_consumed() {
        let mut trigger = Trigger::new();
        let buffer_handler = Arc::new(Mutex::new(Buffer::new().set_max_buffer_size(Some(22))));
        let matcher = Simple::new(r#"[]{"description"}"#).unwrap();

        trigger.add_matcher(Box::new(matcher), buffer_handler.clone());

        // Fits into the buffer
        assert!(trigger.process(br#"[{"description": "short"}, "#).is_ok());
        // Make the buffer shorter
        assert_eq!(
            buffer_handler.lock().unwrap().pop().unwrap(),
            (None, br#""short""#.to_vec())
        );
        assert!(trigger
            .process(br#"{"description": "too long description"}]"#)
            .is_ok());
        // Make the buffer shorter
        assert_eq!(
            buffer_handler.lock().unwrap().pop().unwrap(),
            (None, br#""too long description""#.to_vec())
        );
    }
}
