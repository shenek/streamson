//! Handler which puts output into writeable struct

use super::{Handler, FROMSTR_DELIM};
use crate::{error, path::Path, streamer::Token};
use std::{any::Any, fs, io, str::FromStr};

/// File handler responsible for storing data to a file.
pub struct Output<W>
where
    W: io::Write,
{
    /// writable output
    output: W,

    /// Indicator whether the path will be displayed
    /// e.g. `{"items"}: {"sub": 4}` vs `{"sub": 4}`
    write_path: bool,

    /// String which will be appended to the end of each record
    /// to separate it with the next record (default '#')
    separator: String,
}

impl FromStr for Output<fs::File> {
    type Err = error::Handler;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<_> = input.split(FROMSTR_DELIM).collect();
        match splitted.len() {
            1 => {
                let file = fs::File::create(splitted[0]).map_err(error::Handler::new)?;
                Ok(Self::new(file))
            }
            2 => {
                let file = fs::File::create(splitted[0]).map_err(error::Handler::new)?;
                Ok(Self::new(file)
                    .set_write_path(FromStr::from_str(splitted[1]).map_err(error::Handler::new)?))
            }
            _ => Err(error::Handler::new("Failed to parse")),
        }
    }
}

impl<W> Output<W>
where
    W: io::Write,
{
    /// Creates new Output handler
    ///
    /// # Arguments
    /// * `output` - structur which implements `io::Write`
    ///
    pub fn new(output: W) -> Self {
        Self {
            output,
            write_path: false,
            separator: "\n".into(),
        }
    }

    /// Set whether to show path
    ///
    /// # Arguments
    /// * `use_path` - should path be shown in the output
    ///
    /// # Example
    /// ```
    /// use std::io::stdout;
    /// use streamson_lib::handler;
    /// let output = handler::Output::new(stdout())
    ///     .set_write_path(true);
    /// ```
    pub fn set_write_path(mut self, write_path: bool) -> Self {
        self.write_path = write_path;
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
    /// use std::io::stdout;
    /// use streamson_lib::handler;
    /// let output = handler::Output::new(stdout())
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

impl<W> Handler for Output<W>
where
    W: io::Write + Send + 'static,
{
    fn start(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        _token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if self.write_path {
            self.output
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
        self.output
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
        self.output
            .write(separator.as_bytes())
            .map_err(|err| error::Handler::new(err.to_string()))?;
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any {
        self
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
        handler: handler::Output<fs::File>,
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
        let file = fs::File::create(str_path).unwrap();
        let handler = handler::Output::new(file);

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
        let file = fs::File::create(str_path).unwrap();
        let handler = handler::Output::new(file).set_separator("XXX");

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
        let file = fs::File::create(str_path).unwrap();
        let handler = handler::Output::new(file).set_write_path(true);

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
