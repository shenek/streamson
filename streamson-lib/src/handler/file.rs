use super::Handler;
use crate::error;
use std::{
    fs,
    io::{self, Write},
};

pub struct File {
    file: fs::File,
}

impl File {
    pub fn new(fs_path: &str) -> io::Result<Self> {
        let file = fs::File::create(fs_path)?;
        Ok(Self { file })
    }
}

impl Handler for File {
    fn handle(&mut self, _: &str, data: &[u8]) -> Result<(), error::Generic> {
        self.file.write(data).map_err(|_| error::Generic)?;
        self.file.write(b"\n").map_err(|_| error::Generic)?;
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

    #[test]
    fn existing_file() {
        let file = NamedTempFile::new().unwrap();
        let tmp_path = file.into_temp_path();
        let str_path = tmp_path.to_str().unwrap();

        let matcher = matcher::Simple::new(r#"{"aa"}[]"#);
        let handler = Arc::new(Mutex::new(handler::File::new(str_path).unwrap()));

        let mut collector = Collector::new().add_matcher(Box::new(matcher), &[handler]);

        assert!(collector
            .process(br#"{"aa": [1, 2, "u"], "b": true}"#)
            .unwrap());

        let content = fs::read_to_string(str_path).unwrap();
        assert_eq!(
            content,
            str::from_utf8(
                br#"1
2
"u"
"#
            )
            .unwrap()
        );
    }
}
