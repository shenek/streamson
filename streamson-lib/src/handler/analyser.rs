//! Handler which stores matched paths

use std::collections::HashMap;

use super::Handler;
use crate::{
    error,
    path::{Element, Path},
    streamer::ParsedKind,
};

#[derive(Debug, Default)]
pub struct Analyser {
    /// Stored paths with counts
    paths: HashMap<String, usize>,
}

/// Converts Path to string reducing arrays to "[]"
/// e.g. {"users"}[0]{"name"} => {"users"}[]{"name"}
fn to_recuded_array_str(path: &Path) -> String {
    path.get_path()
        .iter()
        .map(|e| match e {
            Element::Key(key) => format!(r#"{{"{}"}}"#, key),
            Element::Index(_) => "[]".to_string(),
        })
        .collect()
}

impl Handler for Analyser {
    fn handle(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        _data: Option<&[u8]>,
        _kind: ParsedKind,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        *self.paths.entry(to_recuded_array_str(path)).or_insert(0) += 1;
        Ok(None)
    }

    fn use_path(&self) -> bool {
        // paths will be stored
        true
    }

    fn buffering_required(&self) -> bool {
        // no need to buffer input
        false
    }
}

impl Analyser {
    /// Creates a new handler analyser
    /// Which stores paths to analyse the structure of the JSON
    pub fn new() -> Self {
        Self::default()
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
    use crate::{matcher::All, strategy::Trigger};
    use std::sync::{Arc, Mutex};

    #[test]
    fn indexer_handler() {
        let mut trigger = Trigger::new();

        let analyser_handler = Arc::new(Mutex::new(Analyser::new()));

        let matcher = All::default();

        trigger.add_matcher(Box::new(matcher), &[analyser_handler.clone()]);

        trigger.process(br#"{"elements": [1, 2, 3, [41, 42, {"sub1": {"subsub": 1}, "sub2": null}]], "after": true, "last": [{"aaa": 1, "cc": "dd"}, {"aaa": 2, "extra": false}]}"#).unwrap();

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
}
