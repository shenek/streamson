//! Handler which stores matched paths

use std::{any::Any, collections::HashMap, str::FromStr};

use super::Handler;
use crate::{
    error,
    path::{Element, Path},
    streamer::{ParsedKind, Token},
};

#[derive(Debug, Default)]
pub struct Analyser {
    /// Stored paths with counts
    paths: HashMap<String, usize>,
    /// Group by types as well
    group_types: bool,
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
}
