//! Collections of handler (what to do with matched paths and data).

use crate::{error, path::Path, streamer::Output};

pub mod analyser;
pub mod buffer;
pub mod file;
pub mod indexer;
pub mod println;
#[cfg(feature = "with_regex")]
pub mod regex;
pub mod replace;
pub mod shorten;
pub mod unstringify;

pub use self::analyser::Analyser;
pub use self::buffer::Buffer;
pub use self::file::File;
pub use self::indexer::Indexer;
pub use self::println::PrintLn;
#[cfg(feature = "with_regex")]
pub use self::regex::Regex;
pub use self::replace::Replace;
pub use self::shorten::Shorten;
pub use self::unstringify::Unstringify;
pub use crate::streamer::ParsedKind;

pub trait Instance: Send {
    fn finalize(self) -> Result<Option<Vec<u8>>, error::Handler>;
}

/// Common handler trait
pub trait Handler: Send {
    /// Is called when  a path is matched
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `matcher_idx`- idx of matcher which was used
    /// * `token` - part of a input which was matched
    ///
    /// # Returns
    /// * `Ok(None)` - All went well, no output
    /// * `Ok(Some(data))` - All went, handler has some output
    /// * `Err(_)` - Failed to execute handler
    fn start(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Output,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        Ok(None)
    }

    /// Is called when handler receives some data
    ///
    /// # Arguments
    /// * `matcher_idx`- idx of matcher which was used
    /// * `data` - matched data
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
    /// * `token` - part of a input which was matched
    ///
    /// # Returns
    /// * `Ok(None)` - All went well, no data conversion needed
    /// * `Ok(Some(data))` - Alll went well, data converted
    /// * `Err(_)` - Failed to execute handler
    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Output,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        Ok(None)
    }
}
