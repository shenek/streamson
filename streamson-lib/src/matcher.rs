//! Collections of path matchers (matches the path).

use std::fmt;

pub mod combinator;
pub mod depth;
pub mod simple;

pub use combinator::Combinator;
pub use depth::Depth;
pub use simple::Simple;

use crate::path::Path;

/// Common Matcher trait
pub trait MatchMaker: fmt::Debug + Send {
    /// Check whether the path matches
    /// # Arguments
    /// * `path` - path to be matched (has to be a valid path)
    ///
    /// # Returns
    /// * `true` if path matches, `false` otherwise
    fn match_path(&self, path: &Path) -> bool;
}
