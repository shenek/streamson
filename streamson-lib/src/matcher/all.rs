//! Matcher which will simply match all paths
//!
//! should be used only in specific strategies

use std::str::FromStr;

use super::MatchMaker;
use crate::{error, path::Path, streamer::ParsedKind};

/// AllMatch to match array elements
#[derive(Debug, Clone, PartialEq)]
pub struct All;

impl Default for All {
    fn default() -> Self {
        All
    }
}

impl FromStr for All {
    type Err = error::Matcher;
    fn from_str(_: &str) -> Result<Self, Self::Err> {
        Ok(Self)
    }
}

impl MatchMaker for All {
    fn match_path(&self, _: &Path, _: ParsedKind) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{All, MatchMaker};
    use crate::{path::Path, streamer::ParsedKind};
    use std::{convert::TryFrom, str::FromStr};

    #[test]
    fn match_path() {
        let all = All::default();

        assert!(all.match_path(&Path::try_from(r#""#).unwrap(), ParsedKind::Obj));
        assert!(all.match_path(&Path::try_from(r#"{"Any"}"#).unwrap(), ParsedKind::Arr));
        assert!(all.match_path(
            &Path::try_from(r#"{"Any"}[0]{"Any"}"#).unwrap(),
            ParsedKind::Obj
        ));
    }

    #[test]
    fn all_parse() {
        assert!(All::from_str("").is_ok());
        assert!(All::from_str("*").is_ok());
        assert!(All::from_str("all").is_ok());
        assert!(All::from_str(".*").is_ok());
        assert!(All::from_str("any string").is_ok());
    }
}
