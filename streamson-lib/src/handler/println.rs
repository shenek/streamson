//! Handler which puts output into stdout
//!
use super::Handler;
use crate::{error, path::Path};
use std::str;

/// Handler responsible for sending data to stdout.
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
    /// Creates new handler which output items to stdout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to show path
    ///
    /// # Arguments
    /// * `show_path` - should path be shown in the output
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::PrintLn::new().set_show_path(true);
    /// ```
    pub fn set_show_path(mut self, show_path: bool) -> Self {
        self.show_path = show_path;
        self
    }

    /// Set which separator will be used in the output
    ///
    /// Note that every separator will be extended to every found item.
    ///
    /// # Arguments
    /// * `separator` - how found record will be separated
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::PrintLn::new().set_separator("######\n");
    /// ```
    pub fn set_separator<S>(mut self, separator: S) -> Self
    where
        S: ToString,
    {
        self.separator = separator.to_string();
        self
    }
}

impl Handler for PrintLn {
    fn show_path(&self) -> bool {
        self.show_path
    }

    fn separator(&self) -> &str {
        &self.separator
    }

    fn handle(&mut self, path: &Path, data: &[u8]) -> Result<(), error::Handler> {
        let str_data = str::from_utf8(data).map_err(|err| error::Handler::new(err.to_string()))?;
        if self.show_path() {
            print!("{}: {}{}", path, str_data, self.separator());
        } else {
            print!("{}{}", str_data, self.separator());
        }

        Ok(())
    }
}
