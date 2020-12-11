//! Collections of path matchers (matches the path).

use std::fmt;

pub mod all;
pub mod combinator;
pub mod depth;
#[cfg(feature = "with_regex")]
pub mod regex;
pub mod simple;

pub use self::all::All;
pub use self::combinator::Combinator;
pub use self::depth::Depth;
#[cfg(feature = "with_regex")]
pub use self::regex::Regex;
pub use self::simple::Simple;

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
