//! Handler which aggregates matched data
//! and exports it in CSV format

use std::{any::Any, collections::HashMap, mem, num::ParseIntError, str::FromStr};

use super::{Handler, HandlerOutput};
use crate::{
    error,
    path::Path,
    streamer::{ParsedKind, Token},
};

#[derive(Debug, Default)]
pub struct Csv {
    /// Map matcher idx to header name
    header: Vec<(usize, String)>,
    /// Write header to output
    write_header: bool,
    /// Currently processed record
    current: HashMap<usize, String>,
    /// Matcher indexes which should be used
    matcher_indexes: Vec<usize>,
    /// Currently matched data
    matched_path: Option<Path>,
    /// Input buffer
    buffer: Vec<u8>,
    /// Indicator whether csv has data to export
    has_data: bool,
}

fn stringify(input: String) -> String {
    let mut output: String = "\"".to_string();
    for chr in input.chars() {
        if chr == '"' {
            output.push('\\');
        }
        output.push(chr);
    }
    output.push('"');
    output
}

impl Csv {
    pub fn new(header: Vec<(usize, Option<String>)>, write_header: bool) -> Self {
        let header = header
            .into_iter()
            .map(|(matcher_idx, name)| {
                (matcher_idx, name.unwrap_or_else(|| matcher_idx.to_string()))
            })
            .collect::<Vec<(usize, String)>>();

        let matcher_indexes = header.iter().map(|e| e.0).collect();
        Self {
            current: HashMap::new(),
            header,
            write_header,
            matcher_indexes,
            matched_path: None,
            buffer: vec![],
            has_data: false,
        }
    }

    fn convert_to_string(data: Vec<u8>, kind: ParsedKind) -> Result<String, error::Handler> {
        let data_str = String::from_utf8(data).map_err(|e| error::Handler::new(e.to_string()))?;
        match kind {
            ParsedKind::Str => Ok(data_str[1..data_str.len() - 1].to_string()),
            ParsedKind::Null => Ok(String::new()),
            ParsedKind::Bool => Ok(bool::from_str(&data_str)
                .map_err(|e| error::Handler::new(e.to_string()))?
                .to_string()),
            ParsedKind::Num => Ok(data_str),
            ParsedKind::Arr | ParsedKind::Obj => Ok(String::new()),
        }
    }
}

impl FromStr for Csv {
    type Err = error::Handler;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let splitted_indexes: Vec<_> = input.split(',').collect();
        if splitted_indexes.is_empty() {
            return Err(error::Handler::new("Need at least one column"));
        }
        let parsed_indexes: Vec<(usize, Option<String>)> = splitted_indexes
            .into_iter()
            .map(|e| {
                let splitted = e.splitn(2, "-").collect::<Vec<&str>>();
                Ok(match splitted.len() {
                    1 => (splitted[0].parse::<usize>()?, None),
                    2 => (splitted[0].parse::<usize>()?, Some(splitted[1].to_string())),
                    _ => unreachable!(),
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e: ParseIntError| {
                error::Handler::new(format!("Failed to parse matcher number: {}", e))
            })?;

        Ok(Self::new(parsed_indexes, true))
    }
}

impl Handler for Csv {
    fn start(&mut self, path: &Path, _matcher_idx: usize, token: Token) -> HandlerOutput {
        if self.matched_path.is_some() {
            return Ok(None); // only one match at a time
        }

        let mut output: Option<Vec<u8>> = None;

        // make sure header is written
        if self.write_header {
            self.write_header = false;

            let header = self
                .header
                .iter()
                .map(|e| stringify(e.1.to_string()))
                .collect::<Vec<String>>();

            output = Some(
                header
                    .join(",")
                    .as_bytes()
                    .iter()
                    .copied()
                    .collect::<Vec<u8>>(),
            );
        }

        if let Token::Start(_, kind) = token {
            match kind {
                ParsedKind::Obj | ParsedKind::Arr => return Ok(None), // skip structured
                _ => (),
            }
            self.matched_path = Some(path.clone());
            dbg!(&output);
            Ok(output)
        } else {
            unreachable!();
        }
    }

    fn feed(&mut self, data: &[u8], _matcher_idx: usize) -> HandlerOutput {
        if self.matched_path.is_some() {
            self.buffer.extend(data);
        }
        Ok(None)
    }

    fn end(&mut self, path: &Path, matcher_idx: usize, token: Token) -> HandlerOutput {
        if let Some(matched_path) = self.matched_path.take() {
            if &matched_path == path {
                if let Token::End(_, kind) = token {
                    self.has_data = true;
                    let mut buffer = Vec::with_capacity(self.buffer.len());
                    mem::swap(&mut buffer, &mut self.buffer);
                    self.current
                        .insert(matcher_idx, Self::convert_to_string(buffer, kind)?);
                }
            } else {
                self.matched_path = Some(matched_path)
            }
        } else {
            unreachable!();
        }
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn json_finished(&mut self) -> HandlerOutput {
        // Make sure that output is exported only once
        // In case that handler is shared among several matchers
        let mut output: Option<Vec<u8>> = None;
        if self.has_data {
            let indexes = self.matcher_indexes.clone();
            let record = indexes
                .iter()
                .copied()
                .map(|idx| {
                    self.current
                        .remove(&idx)
                        .map(|e| stringify(e))
                        .unwrap_or_else(|| String::new())
                })
                .collect::<Vec<_>>();

            output = Some(record.join(",").as_bytes().iter().copied().collect());
            // make sure that hashmap is cleared
            self.current.clear();
            self.has_data = false;
        }
        dbg!(&output);
        Ok(output)
    }

    fn is_converter(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Csv;
    use crate::{
        handler::{Group, Output},
        matcher::Simple,
        strategy::{Strategy, Trigger},
    };
    use std::{
        io,
        sync::{Arc, Mutex},
    };

    pub struct Buffer {
        pub data: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl io::Write for Buffer {
        fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
            dbg!(buf);
            self.data.lock().unwrap().push(buf.to_vec());
            Ok(buf.len())
        }
        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }

    #[test]
    fn basic() {
        let data = Arc::new(Mutex::new(vec![]));
        let buffer = Buffer { data: data.clone() };
        let mut trigger = Trigger::new();
        let handler1 = Arc::new(Mutex::new(Csv::new(
            vec![
                (0, Some("name".into())),
                (1, Some("street".into())),
                (2, None),
                (3, Some("Opt".into())),
            ],
            true,
        )));
        let handler2 = Arc::new(Mutex::new(Output::new(buffer)));
        let handler = Arc::new(Mutex::new(
            Group::new().add_handler(handler1).add_handler(handler2),
        ));

        trigger.add_matcher(
            Box::new(Simple::new(r#"{"name"}"#).unwrap()),
            handler.clone(),
        );

        trigger.add_matcher(
            Box::new(Simple::new(r#"{"address"}{"street"}"#).unwrap()),
            handler.clone(),
        );

        trigger.add_matcher(
            Box::new(Simple::new(r#"{"age"}"#).unwrap()),
            handler.clone(),
        );

        trigger.add_matcher(
            Box::new(Simple::new(r#"{"opt"}"#).unwrap()),
            handler.clone(),
        );

        assert!(trigger
            .process(br#"{"address": {"street": "s1"}, "name": "user1", "age": 21}"#)
            .is_ok());
        assert!(trigger
            .process(br#"{"address": {"street": "s2"}, "name": "user2", "age": 22}"#)
            .is_ok());
        assert!(trigger
            .process(br#"{"address": {"street": "s3"}, "name": "user3", "age": 23}"#)
            .is_ok());

        let guard = data.lock().unwrap();
        assert_eq!(
            String::from_utf8(guard.iter().fold(vec![], |mut acc: Vec<u8>, x| {
                acc.extend(x);
                acc
            }))
            .unwrap(),
            r#""name","street","2","Opt"
"user1","s1","21",
"user2","s2","22",
"user3","s3","23","#
        );
    }
}
