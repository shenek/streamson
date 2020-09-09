//! Collections of handler (what to do with matched paths and data).

use crate::{error, path::Path, streamer::Output};

pub mod buffer;
pub mod file;
pub mod indexer;
pub mod println;

pub use buffer::Buffer;
pub use file::File;
pub use indexer::Indexer;
pub use println::PrintLn;

/// Common handler trait
pub trait Handler: Send {
    /// Calls handler on splitted data
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `data` - matched data
    /// * `idx`  - input data index of next data after handle
    ///
    /// # Returns
    /// * `Ok(())` - Handler was successfully executed
    /// * `Err(_)` - Failed to execute handler
    ///
    /// # Errors
    ///
    /// Handler failed (e.g. failed to write to output file).
    fn handle(&mut self, path: &Path, data: Option<&[u8]>) -> Result<(), error::Handler>;

    /// Calls when an index occured
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `idx`  - input data index of next data after handle
    fn handle_idx(&mut self, _path: &Path, _idx: Output) -> Result<(), error::Handler> {
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
    /// it can be avioded
    fn buffering_required(&self) -> bool {
        true
    }
}
