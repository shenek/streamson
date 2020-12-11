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

/// Common handler trait
pub trait Handler: Send {
    /// Calls handler on matched data
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `matcher_idx`- idx of matcher which was used
    /// * `data` - matched data
    ///
    /// # Returns
    /// * `Ok(None)` - All went well, no data conversion needed
    /// * `Ok(Some(data))` - Alll went well, data converted
    /// * `Err(_)` - Failed to execute handler
    ///
    /// # Errors
    ///
    /// Handler failed (e.g. failed to write to output file).
    fn handle(
        &mut self,
        path: &Path,
        matcher_idx: usize,
        data: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, error::Handler>;

    /// Calls when an index occured
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `idx`  - input data index of next data after handle
    /// * `matcher_idx`- idx of matcher which was used
    fn handle_idx(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _idx: Output,
    ) -> Result<(), error::Handler> {
        Ok(())
    }

    /// Should path be used
    fn use_path(&self) -> bool {
        false
    }

    /// A str which will be used to separate records
    fn separator(&self) -> &str {
        "\n"
    }

    /// If true is returned a buffer will
    /// be used to store input data when
    /// matcher matches
    ///
    /// Required for most of the handlers,
    /// but there can be situations where,
    /// it can be avoided
    fn buffering_required(&self) -> bool {
        true
    }
}
