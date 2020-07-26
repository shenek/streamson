//! Depth path matcher

use super::MatchMaker;
use crate::path::Path;

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
        let depth = path.depth() - 1; // Skip the Element::Root
        if let Some(max) = self.max {
            self.min <= depth && depth <= max
        } else {
            self.min <= depth
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Depth, MatchMaker};
    use crate::path::Path;
    use std::convert::TryFrom;

    #[test]
    fn match_path() {
        let depth = Depth::new(2, None);

        assert!(!depth.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));

        let depth = Depth::new(2, Some(2));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(depth.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(!depth.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));
    }
}
