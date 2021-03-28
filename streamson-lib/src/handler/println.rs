//! Handler which puts output into stdout
//!
use super::{Handler, FROMSTR_DELIM};
use crate::{error, path::Path, streamer::Token};
use std::{
    any::Any,
    str::{self, FromStr},
};

/// Handler responsible for sending data to stdout.
pub struct PrintLn {
    /// Indicator whether the path will be displayed
    /// e.g. `{"items"}: {"sub": 4}` vs `{"sub": 4}`
    use_path: bool,

    /// String which will be appended to the end of each record
    /// to separate it with the next record (default '#')
    separator: String,

    /// Buffer to store output data (should be printed all at once)
    buffer: Vec<u8>,
}

impl Default for PrintLn {
    fn default() -> Self {
        Self {
            use_path: false,
            separator: "\n".into(),
            buffer: vec![],
        }
    }
}

impl FromStr for PrintLn {
    type Err = error::Handler;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<_> = input.split(FROMSTR_DELIM).collect();
        match splitted.len() {
            0 => Ok(Self::default()),
            1 => Ok(Self::default()
                .set_use_path(FromStr::from_str(splitted[0]).map_err(error::Handler::new)?)),
            2 => Ok(Self::default()
                .set_use_path(FromStr::from_str(splitted[0]).map_err(error::Handler::new)?)
                .set_separator(splitted[1])),
            _ => Err(error::Handler::new("Failed to parse")),
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
    /// * `use_path` - should path be shown in the output
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::PrintLn::new().set_use_path(true);
    /// ```
    pub fn set_use_path(mut self, use_path: bool) -> Self {
        self.use_path = use_path;
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
    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self.buffer.extend(data);
        Ok(None)
    }

    fn end(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if self.use_path {
            print!("{}: ", path);
        }
        let str_data =
            str::from_utf8(&self.buffer).map_err(|err| error::Handler::new(err.to_string()))?;
        print!("{}", str_data);
        print!("{}", self.separator);
        self.buffer.clear();
        Ok(None)
    }

    fn is_converter(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
