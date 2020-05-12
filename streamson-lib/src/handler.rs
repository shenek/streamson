use crate::error;

pub mod println;

pub use println::PrintLn;

pub trait Handler {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic>;
}
