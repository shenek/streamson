//! Collections of handler (what to do with matched paths and data).
//!

pub mod analyser;
pub mod buffer;
pub mod group;
pub mod indenter;
pub mod indexer;
pub mod output;
#[cfg(feature = "with_regex")]
pub mod regex;
pub mod replace;
pub mod shorten;
pub mod unstringify;

use std::any::Any;

use crate::{error, path::Path, streamer::Token};

pub use self::analyser::Analyser;
pub use self::buffer::Buffer;
pub use self::group::Group;
pub use self::indenter::Indenter;
pub use self::indexer::Indexer;
pub use self::output::Output;
#[cfg(feature = "with_regex")]
pub use self::regex::Regex;
pub use self::replace::Replace;
pub use self::shorten::Shorten;
pub use self::unstringify::Unstringify;

/// Shortcut to handler's output
type HandlerOutput = Result<Option<Vec<u8>>, error::Handler>;

/// Common handler trait
pub trait Handler: Send {
    /// Is called when a path is matched
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `matcher_idx`- idx of matcher which was used
    /// * `token` - further info about matched data
    ///
    /// # Returns
    /// * `Ok(None)` - All went well, no output
    /// * `Ok(Some(data))` - All went, handler has some output
    /// * `Err(_)` - Failed to execute handler
    fn start(&mut self, _path: &Path, _matcher_idx: usize, _token: Token) -> HandlerOutput {
        Ok(None)
    }

    /// Is called when handler receives some data
    ///
    /// # Arguments
    /// * `data` - a part of matched data
    /// * `matcher_idx`- idx of matcher which was used
    ///
    /// # Returns
    /// * `Ok(None)` - All went well, no output
    /// * `Ok(Some(data))` - All went, handler has some output
    /// * `Err(_)` - Failed to execute handler
    fn feed(&mut self, _data: &[u8], _matcher_idx: usize) -> HandlerOutput {
        Ok(None)
    }

    /// Is called when the path is no longer matched
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `matcher_idx`- idx of matcher which was used
    /// * `token` - further info about matched data
    ///
    /// # Returns
    /// * `Ok(None)` - All went well, no data conversion needed
    /// * `Ok(Some(data))` - All went well, data converted
    /// * `Err(_)` - Failed to execute handler
    fn end(&mut self, _path: &Path, _matcher_idx: usize, _token: Token) -> HandlerOutput {
        Ok(None)
    }

    /// Should be handler used to convert data
    fn is_converter(&self) -> bool {
        false
    }

    /// Function to allow downcasting
    fn as_any(&self) -> &dyn Any;

    /// Function which is supposed to be called when entire JSON is read
    ///
    /// Note that more than one JSON may be present in the input
    fn json_finished(&mut self) -> HandlerOutput {
        Ok(None)
    }

    /// Function which is supposed to be called when there are no data on input
    fn input_finished(&mut self) -> HandlerOutput {
        Ok(None)
    }
}
