use regex::{self, Error as RegexError};
use std::str::FromStr;

use streamson_lib::{matcher::MatchMaker, path::Path};

/// Regex path matcher
///
/// It uses regex to match path
///
/// # Examples
/// ```
/// use streamson_lib::{handler, Collector};
/// use streamson_extra_matchers::Regex;
///
/// use std::{str::FromStr, sync::{Arc, Mutex}};
///
/// let handler = Arc::new(Mutex::new(handler::PrintLn::new()));
/// let matcher = Regex::from_str(r#"\{"[Uu]ser"\}\[\]"#).unwrap();
///
/// let mut collector = Collector::new();
///
/// collector.add_matcher(
///     Box::new(matcher),
///     &[handler],
/// );
///
/// for input in vec![
///     br#"{"Users": [1,2]"#.to_vec(),
///     br#", "users": [3, 4]}"#.to_vec(),
/// ] {
///     collector.process(&input).unwrap();
/// }
///
/// ```
///
#[derive(Debug, Clone)]
pub struct Regex {
    regex: regex::Regex,
}

impl Regex {
    /// Creates new regex matcher
    ///
    /// # Arguments
    /// * `rgx` - regex structure
    pub fn new(rgx: regex::Regex) -> Self {
        Self { regex: rgx }
    }
}

impl MatchMaker for Regex {
    fn match_path(&self, path: &Path) -> bool {
        let str_path: String = path.to_string();
        self.regex.is_match(&str_path)
    }
}

impl FromStr for Regex {
    type Err = RegexError;
    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let regex = regex::Regex::from_str(path)?;
        Ok(Self::new(regex))
    }
}