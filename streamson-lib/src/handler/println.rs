use super::Handler;
use crate::error;
use std::str;

pub struct PrintLn {
    /// Indicator whether the path will be displayed
    /// e.g. `{"items"}: {"sub": 4}` vs `{"sub": 4}`
    show_path: bool,
}

impl PrintLn {
    pub fn new(show_path: bool) -> Self {
        Self { show_path }
    }
}

/// Prints obtained data to stdout
impl Handler for PrintLn {
    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic> {
        let str_data = str::from_utf8(data).map_err(|_| error::Generic)?;
        if self.show_path {
            println!("{}: {}", path, str_data);
        } else {
            println!("{}", str_data);
        }

        Ok(())
    }
}
