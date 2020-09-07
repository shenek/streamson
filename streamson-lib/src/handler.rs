//! Collections of handler (what to do with matched paths and data).

use crate::{error, path::Path};

pub mod buffer;
pub mod file;
pub mod println;

pub use buffer::Buffer;
pub use file::File;
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
    fn handle(&mut self, path: &Path, data: &[u8], idx: usize) -> Result<(), error::Handler>;

    /// Calls when matcher first matches
    ///
    /// # Arguments
    /// * `path` - path which was matched
    /// * `idx`  - input data index of next data after handle
    fn handle_start(&mut self, _path: &Path, _idx: usize) -> Result<(), error::Handler> {
        Ok(())
    }

    /// Should path be displayed in the output
    fn show_path(&self) -> bool {
        false
    }

    /// A str which will be used to separate records
    fn separator(&self) -> &str {
        "\n"
    }
}
