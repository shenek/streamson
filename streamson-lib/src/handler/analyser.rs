//! Handler which stores matched paths

use std::{any::Any, collections::HashMap, str::FromStr};

use super::{Handler, HandlerOutput};
use crate::{
    error,
    path::{Element, Path},
    streamer::{ParsedKind, Token},
};

pub struct Analyser {
    /// Stored paths with counts
    paths: HashMap<String, usize>,
    /// Group by types as well
    group_types: bool,
    /// Callback which is triggered when input stream finishes
    input_finished_callback: Option<Box<dyn FnMut(&mut Self) + Send>>,
    /// Callback which is triggered entire JSON is processed from input
    json_finished_callback: Option<Box<dyn FnMut(&mut Self) + Send>>,
}

impl Default for Analyser {
    fn default() -> Self {
        Self {
            paths: HashMap::default(),
            group_types: false,
            input_finished_callback: None,
            json_finished_callback: None,
        }
    }
}

/// Converts Path to string reducing arrays to "[]"
/// e.g. {"users"}[0]{"name"} => {"users"}[]{"name"}
fn to_recuded_array_str(path: &Path, kind: Option<ParsedKind>) -> String {
    let mut res: String = path
        .get_path()
        .iter()
        .map(|e| match e {
            Element::Key(key) => format!(r#"{{"{}"}}"#, key),
            Element::Index(_) => "[]".to_string(),
        })
        .collect();
    if let Some(kind) = kind {
        res.push_str(&format!("<{}>", kind.as_ref()));
    }
    res
}

impl Handler for Analyser {
    fn start(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        if let Token::Start(_, kind) = token {
            *self
                .paths
                .entry(to_recuded_array_str(
                    path,
                    if self.group_types { Some(kind) } else { None },
                ))
                .or_insert(0) += 1;
        } else {
            unreachable!();
        }
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn input_finished(&mut self) -> HandlerOutput {
        if let Some(mut callback) = self.input_finished_callback.take() {
            callback(self);
            self.input_finished_callback = Some(callback);
        }
        Ok(None)
    }

    fn json_finished(&mut self) -> HandlerOutput {
        if let Some(mut callback) = self.json_finished_callback.take() {
            callback(self);
            self.json_finished_callback = Some(callback);
        }
        Ok(None)
    }
}

impl FromStr for Analyser {
    type Err = error::Handler;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.is_empty() {
            Ok(Self::default())
        } else {
            let group_types: bool =
                bool::from_str(input).map_err(|e| Self::Err::new(e.to_string()))?;
            Ok(Self::default().set_group_types(group_types))
        }
    }
}

impl Analyser {
    /// Creates a new handler analyser
    /// Which stores paths to analyse the structure of the JSON
    ///
    /// # Arguments
    /// * group_types - should be grouped using types an well as paths
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets group types option
    pub fn set_group_types(mut self, group_types: bool) -> Self {
        self.group_types = group_types;
        self
    }

    /// Results of analysis
    pub fn results(&self) -> Vec<(String, usize)> {
        let mut res: Vec<(String, usize)> = self
            .paths
            .iter()
            .map(|(path, count)| (path.to_string(), *count))
            .collect();
        res.sort_by(|(a_path, _), (b_path, _)| a_path.cmp(b_path));
        res
    }

    /// Adds a callback handler which is triggered entire input is processed
    pub fn set_input_finished_callback(
        &mut self,
        callback: Option<Box<dyn FnMut(&mut Self) + Send>>,
    ) {
        self.input_finished_callback = callback;
    }

    /// Adds a callback handler which is triggered entire JSON is read from input
    pub fn set_json_finished_callback(
        &mut self,
        callback: Option<Box<dyn FnMut(&mut Self) + Send>>,
    ) {
        self.json_finished_callback = callback;
    }
}

#[cfg(test)]
mod tests {
    use super::Analyser;
    use crate::strategy::{All, Strategy};
    use std::sync::{Arc, Mutex};

    #[test]
    fn analyser_handler() {
        let mut all = All::new();

        let analyser_handler = Arc::new(Mutex::new(Analyser::new()));

        all.add_handler(analyser_handler.clone());

        all.process(br#"{"elements": [1, 2, 3, [41, 42, {"sub1": {"subsub": 1}, "sub2": null}]], "after": true, "last": [{"aaa": 1, "cc": "dd"}, {"aaa": 2, "extra": false}]}"#).unwrap();

        // Test analyser handler
        let results = analyser_handler.lock().unwrap().results();
        assert_eq!(results.len(), 13);
        assert_eq!(results[0], ("".to_string(), 1));
        assert_eq!(results[1], (r#"{"after"}"#.to_string(), 1));
        assert_eq!(results[2], (r#"{"elements"}"#.to_string(), 1));
        assert_eq!(results[3], (r#"{"elements"}[]"#.to_string(), 4));
        assert_eq!(results[4], (r#"{"elements"}[][]"#.to_string(), 3));
        assert_eq!(results[5], (r#"{"elements"}[][]{"sub1"}"#.to_string(), 1));
        assert_eq!(
            results[6],
            (r#"{"elements"}[][]{"sub1"}{"subsub"}"#.to_string(), 1)
        );
        assert_eq!(results[7], (r#"{"elements"}[][]{"sub2"}"#.to_string(), 1));
        assert_eq!(results[8], (r#"{"last"}"#.to_string(), 1));
        assert_eq!(results[9], (r#"{"last"}[]"#.to_string(), 2));
        assert_eq!(results[10], (r#"{"last"}[]{"aaa"}"#.to_string(), 2));
        assert_eq!(results[11], (r#"{"last"}[]{"cc"}"#.to_string(), 1));
        assert_eq!(results[12], (r#"{"last"}[]{"extra"}"#.to_string(), 1));
    }

    #[test]
    fn analyser_handler_with_types() {
        let mut all = All::new();

        let analyser_handler = Arc::new(Mutex::new(Analyser::new().set_group_types(true)));

        all.add_handler(analyser_handler.clone());

        all.process(br#"{"elements": [1, 2, 3, [41, 42, {"sub1": {"subsub": 1}, "sub2": null}]], "after": true, "last": [{"aaa": 1, "cc": "dd"}, {"aaa": 2, "extra": false}]}"#).unwrap();

        // Test analyser handler
        let results = analyser_handler.lock().unwrap().results();
        assert_eq!(results.len(), 15);
        assert_eq!(results[0], ("<object>".to_string(), 1));
        assert_eq!(results[1], (r#"{"after"}<boolean>"#.to_string(), 1));
        assert_eq!(results[2], (r#"{"elements"}<array>"#.to_string(), 1));
        assert_eq!(results[3], (r#"{"elements"}[]<array>"#.to_string(), 1));
        assert_eq!(results[4], (r#"{"elements"}[]<number>"#.to_string(), 3));
        assert_eq!(results[5], (r#"{"elements"}[][]<number>"#.to_string(), 2));
        assert_eq!(results[6], (r#"{"elements"}[][]<object>"#.to_string(), 1));
        assert_eq!(
            results[7],
            (r#"{"elements"}[][]{"sub1"}<object>"#.to_string(), 1)
        );
        assert_eq!(
            results[8],
            (
                r#"{"elements"}[][]{"sub1"}{"subsub"}<number>"#.to_string(),
                1
            )
        );
        assert_eq!(
            results[9],
            (r#"{"elements"}[][]{"sub2"}<null>"#.to_string(), 1)
        );
        assert_eq!(results[10], (r#"{"last"}<array>"#.to_string(), 1));
        assert_eq!(results[11], (r#"{"last"}[]<object>"#.to_string(), 2));
        assert_eq!(results[12], (r#"{"last"}[]{"aaa"}<number>"#.to_string(), 2));
        assert_eq!(results[13], (r#"{"last"}[]{"cc"}<string>"#.to_string(), 1));
        assert_eq!(
            results[14],
            (r#"{"last"}[]{"extra"}<boolean>"#.to_string(), 1)
        );
    }

    #[test]
    fn callbacks() {
        let mut all = All::new();

        let first_and_last = Arc::new(Mutex::new(Box::new(vec![])));
        let cloned = first_and_last.clone();
        let mut handler = Analyser::new();
        handler.set_input_finished_callback(Some(Box::new(move |h: &mut Analyser| {
            let results = h.results();
            cloned.lock().unwrap().push(results[0].clone());
            cloned
                .lock()
                .unwrap()
                .push(results[results.len() - 1].clone());
        })));

        let lengths = Arc::new(Mutex::new(Box::new(vec![])));
        let cloned = lengths.clone();
        handler.set_json_finished_callback(Some(Box::new(move |h: &mut Analyser| {
            cloned.lock().unwrap().push(h.results().len());
        })));

        let handler = Arc::new(Mutex::new(handler));
        all.add_handler(handler.clone());
        all.process(br#"{"elements": [1, 2, 3, [41, 42, {"sub1": {"subsub": 1}, "sub2": null}]], "after": true, "last": [{"aaa": 1, "cc": "dd"}, {"aaa": 2, "extra": false}]}"#).unwrap();
        all.process(br#"{"elements": [1, 2, 3, [41, 42, {"sub1": {"subsub": 1}, "sub2": null}]], "after": true, "last": [{"aaa": 1, "cc": "dd"}, {"aaa": 2, "extra": false}]}"#).unwrap();
        all.terminate().unwrap();

        // Test finished nahdler analyser handler
        let guard = first_and_last.lock().unwrap();
        let results: Vec<_> = guard.iter().collect();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], &(r#""#.to_string(), 2));
        assert_eq!(results[1], &(r#"{"last"}[]{"extra"}"#.to_string(), 2));

        let guard = lengths.lock().unwrap();
        let lengths: Vec<_> = guard.iter().copied().collect();
        assert_eq!(lengths.len(), 2);
        assert_eq!(lengths[0], 13);
        assert_eq!(lengths[1], 13);
    }
}
