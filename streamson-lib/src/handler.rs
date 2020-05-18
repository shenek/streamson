use crate::error;

pub mod file;
pub mod println;

pub use file::File;
pub use println::PrintLn;

pub trait Handler {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic>;

    /// Should path be displayed in the output
    fn show_path(&self) -> bool {
        false
    }

    /// A str which will be used to separate records
    fn separator(&self) -> &str {
        "\n"
    }
}
