use super::Handler;
use crate::error;
use std::str;

pub struct PrintLn {
    /// Indicator whether the path will be displayed
    /// e.g. `{"items"}: {"sub": 4}` vs `{"sub": 4}`
    show_path: bool,

    /// String which will be appended to the end of each record
    /// to separate it with the next record (default '#')
    separator: String,
}

impl Default for PrintLn {
    fn default() -> Self {
        Self {
            show_path: false,
            separator: "\n".into(),
        }
    }
}

impl PrintLn {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_show_path(mut self, show_path: bool) -> Self {
        self.show_path = show_path;
        self
    }

    pub fn set_separator<S>(mut self, separator: S) -> Self
    where
        S: ToString,
    {
        self.separator = separator.to_string();
        self
    }
}

/// Prints obtained data to stdout
impl Handler for PrintLn {
    fn show_path(&self) -> bool {
        self.show_path
    }

    fn separator(&self) -> &str {
        &self.separator
    }

    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic> {
        let str_data = str::from_utf8(data).map_err(|_| error::Generic)?;
        if self.show_path() {
            print!("{}: {}{}", path, str_data, self.separator());
        } else {
            print!("{}{}", str_data, self.separator());
        }

        Ok(())
    }
}
