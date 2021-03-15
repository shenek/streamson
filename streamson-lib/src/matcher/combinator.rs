//! Combinator path matcher

use std::{ops, sync::Arc};

use super::Matcher;
use crate::{path::Path, streamer::ParsedKind};

#[derive(Debug, Clone)]
/// Combines several matches together
///
/// It implements normal boolean algebra
/// * `! comb`  will negate the combinator
/// * `comb1 & comb2` both should pass
/// * `comb1 | comb2` at least one should pass
pub enum Combinator {
    /// Represents the actual underlying matcher
    Matcher(Arc<dyn Matcher + Sync>),
    /// Negates the expression
    Not(Box<Combinator>),
    /// Both expressions should be valid
    And(Box<Combinator>, Box<Combinator>),
    /// At least one of the expressions should be valid
    Or(Box<Combinator>, Box<Combinator>),
}

impl Matcher for Combinator {
    fn match_path(&self, path: &Path, kind: ParsedKind) -> bool {
        match self {
            Self::Matcher(matcher) => matcher.match_path(path, kind),
            Self::Not(combinator) => !combinator.match_path(path, kind),
            Self::Or(first, second) => {
                first.match_path(path, kind) || second.match_path(path, kind)
            }
            Self::And(first, second) => {
                first.match_path(path, kind) && second.match_path(path, kind)
            }
        }
    }
}

impl Combinator {
    /// Creates a new matcher combinator
    ///
    /// # Arguments
    /// * `matcher` - matcher to be wrapped
    pub fn new(matcher: impl Matcher + 'static + Sync) -> Self {
        Self::Matcher(Arc::new(matcher))
    }
}

impl ops::Not for Combinator {
    type Output = Self;

    fn not(self) -> Self {
        Self::Not(Box::new(self))
    }
}

impl ops::BitAnd for Combinator {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self::And(Box::new(self), Box::new(rhs))
    }
}

impl ops::BitOr for Combinator {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self::Or(Box::new(self), Box::new(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::{Combinator, Matcher};
    use crate::{
        matcher::{Depth, Simple},
        path::Path,
        streamer::ParsedKind,
    };
    use std::convert::TryFrom;

    #[test]
    fn wrapper() {
        let comb = Combinator::new(Depth::new(1, Some(1)));
        assert!(comb.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(comb.match_path(&Path::try_from(r#"[0]"#).unwrap(), ParsedKind::Obj));
    }

    #[test]
    fn not() {
        let comb = !Combinator::new(Depth::new(1, None));
        assert!(!comb.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(!comb.match_path(&Path::try_from(r#"[0]"#).unwrap(), ParsedKind::Obj));
        assert!(comb.match_path(&Path::try_from(r#""#).unwrap(), ParsedKind::Obj));
        assert!(!comb.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
    }

    #[test]
    fn and() {
        let comb = Combinator::new(Depth::new(1, Some(1)))
            & Combinator::new(Simple::new(r#"{}"#).unwrap());
        assert!(comb.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(!comb.match_path(&Path::try_from(r#"[0]"#).unwrap(), ParsedKind::Obj));
        assert!(!comb.match_path(&Path::try_from(r#""#).unwrap(), ParsedKind::Obj));
        assert!(!comb.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
    }

    #[test]
    fn or() {
        let comb = Combinator::new(Depth::new(1, Some(1)))
            | Combinator::new(Simple::new(r#"{}[0]"#).unwrap());
        assert!(comb.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(comb.match_path(&Path::try_from(r#"[0]"#).unwrap(), ParsedKind::Obj));
        assert!(!comb.match_path(&Path::try_from(r#""#).unwrap(), ParsedKind::Obj));
        assert!(comb.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!comb.match_path(
            &Path::try_from(r#"{"People"}[1]"#).unwrap(),
            ParsedKind::Obj
        ));
    }

    #[test]
    fn complex() {
        let comb1 = Combinator::new(Depth::new(1, Some(1)))
            | Combinator::new(Simple::new(r#"{}[0]"#).unwrap());
        let comb2 = Combinator::new(Depth::new(2, Some(2)))
            | Combinator::new(Simple::new(r#"{}"#).unwrap());
        let comb3 = !comb1 & comb2;

        assert!(!comb3.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(!comb3.match_path(&Path::try_from(r#"[0]"#).unwrap(), ParsedKind::Obj));
        assert!(!comb3.match_path(&Path::try_from(r#""#).unwrap(), ParsedKind::Obj));
        assert!(!comb3.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(comb3.match_path(
            &Path::try_from(r#"{"People"}[1]"#).unwrap(),
            ParsedKind::Obj
        ));
    }
}
