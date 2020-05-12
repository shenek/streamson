use super::Handler;
use crate::error;
use std::str;

pub struct PrintLn;

impl Handler for PrintLn {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic> {
        println!(
            "{}: {}",
            path,
            str::from_utf8(data).map_err(|_| error::Generic)?
        );

        Ok(())
    }
}
