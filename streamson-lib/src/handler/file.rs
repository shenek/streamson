use super::Handler;
use crate::error;
use std::{
    fs,
    io::{self, Write},
};

pub struct File {
    file: fs::File,
    show_path: bool,
    separator: String,
}

impl File {
    pub fn new(fs_path: &str) -> io::Result<Self> {
        let file = fs::File::create(fs_path)?;
        Ok(Self {
            file,
            show_path: false,
            separator: "\n".into(),
        })
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

impl Handler for File {
    fn show_path(&self) -> bool {
        self.show_path
    }

    fn separator(&self) -> &str {
        &self.separator
    }

    fn handle(&mut self, path: &str, data: &[u8]) -> Result<(), error::Generic> {
        if self.show_path {
            self.file
                .write(format!("{}: ", path).as_bytes())
                .map_err(|_| error::Generic)?;
        }
        self.file.write(data).map_err(|_| error::Generic)?;
        let separator = self.separator().to_string();
        self.file
            .write(separator.as_bytes())
            .map_err(|_| error::Generic)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{handler, matcher, Collector};
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
        let mut collector = Collector::new().add_matcher(Box::new(matcher), &[handler]);

        assert!(collector.process(input).unwrap());
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn basic() {
        let tmp_path = NamedTempFile::new().unwrap().into_temp_path();
        let str_path = tmp_path.to_str().unwrap();

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#);
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

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#);
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
    fn show_path() {
        let tmp_path = NamedTempFile::new().unwrap().into_temp_path();
        let str_path = tmp_path.to_str().unwrap();

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#);
        let handler = handler::File::new(str_path).unwrap().set_show_path(true);

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
