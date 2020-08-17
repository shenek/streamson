//! Depth path matcher

use std::str::FromStr;

use super::MatchMaker;
use crate::{error, path::Path};

/// Based on actual path depth
///
/// Path is matched when path depth is higher or equal min and lower or equal max (optional)
#[derive(Default, Debug, Clone)]
pub struct Depth {
    min: usize,
    max: Option<usize>,
}

impl Depth {
    /// Creates new depth matcher
    ///
    /// # Arguments
    /// * `min` - minimal depth (lower won't be matched)
    /// * `max` - maximal depth - optional (higher won't be matched)
    pub fn new(min: usize, max: Option<usize>) -> Self {
        Self { min, max }
    }
}

impl MatchMaker for Depth {
    fn match_path(&self, path: &Path) -> bool {
        let depth = path.depth();
        if let Some(max) = self.max {
            self.min <= depth && depth <= max
        } else {
            self.min <= depth
        }
    }
}

impl FromStr for Depth {
    type Err = error::Matcher;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.splitn(2, '-').collect();
        if splitted.len() == 2 {
            match (splitted[0].parse(), splitted[1].parse()) {
                (Ok(start), Ok(end)) => {
                    if start > end {
                        Err(error::Matcher::Parse(s.into()))
                    } else {
                        Ok(Self::new(start, Some(end)))
                    }
                }
                (Ok(start), _) if splitted[1].is_empty() => Ok(Self::new(start, None)),
                _ => Err(error::Matcher::Parse(s.into())),
            }
        } else {
            Err(error::Matcher::Parse(s.into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Depth, MatchMaker};
    use crate::path::Path;
    use std::{convert::TryFrom, str::FromStr};

    #[test]
    fn match_path() {
        let depth = Depth::from_str("2-").unwrap();

        assert!(!depth.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));

        let depth: Depth = "2-2".parse().unwrap();
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));
    }

    #[test]
    fn depth_parse() {
        assert!(Depth::from_str("").is_err());
        assert!(Depth::from_str("-").is_err());
        assert!(Depth::from_str("4-").is_ok());
        assert!(Depth::from_str("4-5").is_ok());
        assert!(Depth::from_str("4-4").is_ok());
        assert!(Depth::from_str("4-3").is_err());
        assert!(Depth::from_str("4-3x").is_err());
    }
}
