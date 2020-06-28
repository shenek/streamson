//! Combinator path matcher

use super::MatchMaker;
use std::ops;
use std::sync::Arc;

#[derive(Debug, Clone)]
/// Combines several matches together
///
/// It implements normal boolean algebra
/// * `! comb`  will negate the combinator
/// * `comb1 & comb2` both should pass
/// * `comb1 | comb2` at least one should pass
pub enum Combinator {
    /// Represents the actual underlying matcher
    Matcher(Arc<dyn MatchMaker + Sync>),
    /// Negates the expression
    Not(Box<Combinator>),
    /// Both expressions should be valid
    And(Box<Combinator>, Box<Combinator>),
    /// At least one of the expressions should be valid
    Or(Box<Combinator>, Box<Combinator>),
}

impl MatchMaker for Combinator {
    fn match_path(&self, path: &str) -> bool {
        match self {
            Self::Matcher(matcher) => matcher.match_path(path),
            Self::Not(combinator) => !combinator.match_path(path),
            Self::Or(first, second) => first.match_path(path) || second.match_path(path),
            Self::And(first, second) => first.match_path(path) && second.match_path(path),
        }
    }
}

impl Combinator {
    /// Creates a new matcher combinator
    ///
    /// # Arguments
    /// * `matcher` - matcher to be wrapped
    pub fn new(matcher: impl MatchMaker + 'static + Sync) -> Self {
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
    use super::{Combinator, MatchMaker};
    use crate::matcher::{Depth, Simple};

    #[test]
    fn wrapper() {
        let comb = Combinator::new(Depth::new(1, Some(1)));
        assert!(comb.match_path(r#"{"People"}"#));
        assert!(comb.match_path(r#"[0]"#));
    }

    #[test]
    fn not() {
        let comb = !Combinator::new(Depth::new(1, None));
        assert!(!comb.match_path(r#"{"People"}"#));
        assert!(!comb.match_path(r#"[0]"#));
        assert!(comb.match_path(r#""#));
        assert!(!comb.match_path(r#"{"People"}[0]"#));
    }

    #[test]
    fn and() {
        let comb = Combinator::new(Depth::new(1, Some(1))) & Combinator::new(Simple::new(r#"{}"#));
        assert!(comb.match_path(r#"{"People"}"#));
        assert!(!comb.match_path(r#"[0]"#));
        assert!(!comb.match_path(r#""#));
        assert!(!comb.match_path(r#"{"People"}[0]"#));
    }

    #[test]
    fn or() {
        let comb =
            Combinator::new(Depth::new(1, Some(1))) | Combinator::new(Simple::new(r#"{}[0]"#));
        assert!(comb.match_path(r#"{"People"}"#));
        assert!(comb.match_path(r#"[0]"#));
        assert!(!comb.match_path(r#""#));
        assert!(comb.match_path(r#"{"People"}[0]"#));
        assert!(!comb.match_path(r#"{"People"}[1]"#));
    }

    #[test]
    fn complex() {
        let comb1 =
            Combinator::new(Depth::new(1, Some(1))) | Combinator::new(Simple::new(r#"{}[0]"#));
        let comb2 = Combinator::new(Depth::new(2, Some(2))) | Combinator::new(Simple::new(r#"{}"#));
        let comb3 = !comb1 & comb2;

        assert!(!comb3.match_path(r#"{"People"}"#));
        assert!(!comb3.match_path(r#"[0]"#));
        assert!(!comb3.match_path(r#""#));
        assert!(!comb3.match_path(r#"{"People"}[0]"#));
        assert!(comb3.match_path(r#"{"People"}[1]"#));
    }
}
