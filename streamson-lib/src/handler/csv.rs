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
    /// Was header written
    header_written: bool,
    /// Currently processed record
    current: HashMap<usize, String>,
    /// Matcher indexes which should be used
    matcher_indexes: Vec<usize>,
    /// Output buffer
    output: Vec<Vec<String>>,
    /// Currently matched data
    matched_path: Option<Path>,
    /// Input buffer
    buffer: Vec<u8>,
    /// Indicator whether csv has data to export
    has_data: bool,
}

impl Csv {
    pub fn new(header: Vec<(usize, Option<String>)>) -> Self {
        let header = header
            .into_iter()
            .map(|(size, name)| (size, name.unwrap_or_else(|| size.to_string())))
            .collect::<Vec<(usize, String)>>();

        let matcher_indexes = header.iter().map(|e| e.0).collect();
        Self {
            header_written: false,
            current: HashMap::new(),
            header,
            matcher_indexes,
            output: vec![],
            matched_path: None,
            buffer: vec![],
            has_data: false,
        }
    }

    pub fn skip_header(mut self) -> Self {
        self.header_written = true;
        self
    }

    pub fn pop(&mut self) -> Option<Vec<String>> {
        self.output.pop()
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

        Ok(Self::new(parsed_indexes))
    }
}

impl Handler for Csv {
    fn start(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if self.matched_path.is_some() {
            return Ok(None); // only one match at a time
        }
        if let Token::Start(_, kind) = token {
            match kind {
                ParsedKind::Obj | ParsedKind::Arr => return Ok(None), // skip structured
                _ => (),
            }
            self.matched_path = Some(path.clone());
            Ok(None)
        } else {
            unreachable!();
        }
    }

    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if self.matched_path.is_some() {
            self.buffer.extend(data);
        }
        Ok(None)
    }

    fn end(
        &mut self,
        path: &Path,
        matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if let Some(matched_path) = self.matched_path.take() {
            if &matched_path == path {
                if let Token::End(_, kind) = token {
                    self.has_data = true;
                    let mut buffer = Vec::with_capacity(self.buffer.len());
                    mem::swap(&mut buffer, &mut self.buffer);
                    self.current
                        .insert(matcher_idx, Csv::convert_to_string(buffer, kind)?);
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
        if self.has_data {
            let indexes = self.matcher_indexes.clone();
            let record = indexes
                .iter()
                .copied()
                .map(|idx| self.current.remove(&idx).unwrap_or_else(|| String::new()))
                .collect();

            self.output.push(record);

            // make sure that hashmap is cleared
            self.current.clear();
            self.has_data = false;
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::Csv;
    use crate::{
        matcher::Simple,
        strategy::{Strategy, Trigger},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn basic() {
        let mut trigger = Trigger::new();
        let handler = Arc::new(Mutex::new(Csv::new(vec![
            (0, Some("name".into())),
            (1, Some("street".into())),
            (2, None),
        ])));

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

        assert!(trigger
            .process(br#"{"address": {"street": "s1"}, "name": "user1", "age": 21}"#)
            .is_ok());
        assert!(trigger
            .process(br#"{"address": {"street": "s2"}, "name": "user2", "age": 22}"#)
            .is_ok());
        assert!(trigger
            .process(br#"{"address": {"street": "s3"}, "name": "user3", "age": 23}"#)
            .is_ok());

        let mut guard = handler.lock().unwrap();
        assert_eq!(
            guard.pop().unwrap(),
            vec!["user3".to_string(), "s3".to_string(), "23".to_string()]
        );
        assert_eq!(
            guard.pop().unwrap(),
            vec!["user2".to_string(), "s2".to_string(), "22".to_string()]
        );
        assert_eq!(
            guard.pop().unwrap(),
            vec!["user1".to_string(), "s1".to_string(), "21".to_string()]
        );
        assert_eq!(guard.pop(), None);
    }
}
