//! Collections of handler (what to do with matched paths and data).
//!

pub mod analyser;
pub mod buffer;
pub mod file;
pub mod group;
pub mod indenter;
pub mod indexer;
pub mod println;
#[cfg(feature = "with_regex")]
pub mod regex;
pub mod replace;
pub mod shorten;
pub mod unstringify;

use crate::{error, path::Path, streamer::Token};

pub use self::analyser::Analyser;
pub use self::buffer::Buffer;
pub use self::file::File;
pub use self::group::Group;
pub use self::indenter::Indenter;
pub use self::indexer::Indexer;
pub use self::println::PrintLn;
#[cfg(feature = "with_regex")]
pub use self::regex::Regex;
pub use self::replace::Replace;
pub use self::shorten::Shorten;
pub use self::unstringify::Unstringify;

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
    fn start(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
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
    fn feed(
        &mut self,
        _data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
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
    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        Ok(None)
    }

    /// Should be handler used to convert data
    fn is_converter(&self) -> bool {
        false
    }
}
