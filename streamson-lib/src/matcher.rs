pub mod simple;

pub use simple::Simple;

/// Common Matcher trait
pub trait MatchMaker {
    /// Check whether the path matches
    /// # Arguments
    /// * `path` - path to be matched
    ///
    /// # Returns
    /// * `true` if path matches, `false` otherwise
    fn match_path(&self, path: &str) -> bool;
}
