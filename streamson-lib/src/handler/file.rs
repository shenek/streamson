//! Handler which puts output into a file

use super::Handler;
use crate::{error, path::Path, streamer::Token};
use std::{fs, io::Write};

/// File handler responsible for storing data to a file.
pub struct File {
    /// Opened file structure for storing output
    file: fs::File,

    /// Indicator whether the path will be displayed
    /// e.g. `{"items"}: {"sub": 4}` vs `{"sub": 4}`
    use_path: bool,

    /// String which will be appended to the end of each record
    /// to separate it with the next record (default '#')
    separator: String,
}

impl File {
    /// Creates new File handler
    ///
    /// # Arguments
    /// * `fs_path` - path to a file in the file system (will be truncated)
    ///
    /// # Returns
    /// * `Ok(File)` - Handler was successfully created
    /// * `Err(_)` - Failed to create handler
    ///
    /// # Errors
    ///
    /// Error might occur when the file fails to open
    pub fn new(fs_path: &str) -> Result<Self, error::Handler> {
        let file = fs::File::create(fs_path).map_err(|err| error::Handler::new(err.to_string()))?;
        Ok(Self {
            file,
            use_path: false,
            separator: "\n".into(),
        })
    }

    /// Set whether to show path
    ///
    /// # Arguments
    /// * `use_path` - should path be shown in the output
    ///
    /// # Example
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::File::new("output.txt")
    ///     .unwrap()
    ///     .set_use_path(true);
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
    /// ```
    /// use streamson_lib::handler;
    /// let file = handler::File::new("output.txt")
    ///     .unwrap()
    ///     .set_separator("######\n");
    /// ```
    pub fn set_separator<S>(mut self, separator: S) -> Self
    where
        S: ToString,
    {
        self.separator = separator.to_string();
        self
    }
}

impl Handler for File {
    fn start(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if self.use_path {
            self.file
                .write(format!("{}: ", path).as_bytes())
                .map_err(|err| error::Handler::new(err.to_string()))?;
        }
        Ok(None)
    }

    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        self.file
            .write(data)
            .map_err(|err| error::Handler::new(err.to_string()))?;
        Ok(None)
    }

    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let separator = self.separator.to_string();
        self.file
            .write(separator.as_bytes())
            .map_err(|err| error::Handler::new(err.to_string()))?;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        handler, matcher,
        strategy::{self, Strategy},
    };
    use std::{
        fs, str,
        sync::{Arc, Mutex},
    };
    use tempfile::NamedTempFile;

    fn make_output(
        path: &str,
        matcher: matcher::Simple,
        handler: handler::File,
        input: &[u8],
    ) -> String {
        let handler = Arc::new(Mutex::new(handler));
        let mut trigger = strategy::Trigger::new();
        trigger.add_matcher(Box::new(matcher), handler);

        trigger.process(input).unwrap();
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn basic() {
        let tmp_path = NamedTempFile::new().unwrap().into_temp_path();
        let str_path = tmp_path.to_str().unwrap();

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#).unwrap();
        let handler = handler::File::new(str_path).unwrap();

        let output = make_output(
            str_path,
            matcher,
            handler,
            br#"{"aa": [1, 2, "u"], "b": true}"#,
        );

        assert_eq!(
            output,
            str::from_utf8(
                br#"1
2
"u"
"#
            )
            .unwrap()
        );
    }

    #[test]
    fn separator() {
        let tmp_path = NamedTempFile::new().unwrap().into_temp_path();
        let str_path = tmp_path.to_str().unwrap();

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#).unwrap();
        let handler = handler::File::new(str_path).unwrap().set_separator("XXX");

        let output = make_output(
            str_path,
            matcher,
            handler,
            br#"{"aa": [1, 2, "u"], "b": true}"#,
        );

        assert_eq!(output, str::from_utf8(br#"1XXX2XXX"u"XXX"#).unwrap());
    }

    #[test]
    fn use_path() {
        let tmp_path = NamedTempFile::new().unwrap().into_temp_path();
        let str_path = tmp_path.to_str().unwrap();

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#).unwrap();
        let handler = handler::File::new(str_path).unwrap().set_use_path(true);

        let output = make_output(
            str_path,
            matcher,
            handler,
            br#"{"aa": [1, 2, "u"], "b": true}"#,
        );

        assert_eq!(
            output,
            str::from_utf8(
                br#"{"aa"}[0]: 1
{"aa"}[1]: 2
{"aa"}[2]: "u"
"#
            )
            .unwrap()
        );
    }
}
