//! Collections of path matchers (matches the path).

pub mod depth;
pub mod simple;

pub use depth::Depth;
pub use simple::Simple;

/// Common Matcher trait
pub trait MatchMaker {
    /// Check whether the path matches
    /// # Arguments
    /// * `path` - path to be matched (has to be a valid path)
    ///
    /// # Returns
    /// * `true` if path matches, `false` otherwise
    fn match_path(&self, path: &str) -> bool;
}
