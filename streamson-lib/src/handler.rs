use crate::error::GenericError;

pub mod println;

pub use println::PrintLn;

pub trait Handler {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), GenericError>;
}
