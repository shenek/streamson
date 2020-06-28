//! Collections of handler (what to do with matched paths and data).

use crate::error;

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
    ///
    /// # Returns
    /// * `Ok(())` - Handler was successfully executed
    /// * `Err(_)` - Failed to execute handler
    ///
    /// # Errors
    ///
    /// Handler failed (e.g. failed to write to output file).
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Handler>;

    /// Should path be displayed in the output
    fn show_path(&self) -> bool {
        false
    }

    /// A str which will be used to separate records
    fn separator(&self) -> &str {
        "\n"
    }
}
