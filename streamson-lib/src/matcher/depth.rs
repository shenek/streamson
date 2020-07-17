//! Depth path matcher

use super::MatchMaker;

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
    fn match_path(&self, path: &str) -> bool {
        let mut escaped: bool = false;
        let mut depth: usize = 0;
        let max_num = if let Some(m) = self.max {
            m
        } else {
            usize::MAX
        };
        for chr in path.chars() {
            match chr {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '}' | ']' => {
                    depth += 1;
                    if self.max.is_none() {
                        if depth >= self.min {
                            return true;
                        }
                    } else if depth > max_num {
                        return false;
                    }
                }
                _ => (),
            }
        }
        depth >= self.min
    }
}

#[cfg(test)]
mod tests {
    use super::{Depth, MatchMaker};

    #[test]
    fn match_path() {
        let depth = Depth::new(2, None);

        assert!(!depth.match_path(r#"{"People"}"#));
        assert!(depth.match_path(r#"{"People"}[0]"#));
        assert!(depth.match_path(r#"{"People"}[0]{"Age"}"#));
        assert!(depth.match_path(r#"{"People"}[0]{"Height"}"#));
        assert!(depth.match_path(r#"{"People"}[1]"#));
        assert!(depth.match_path(r#"{"People"}[1]{"Age"}"#));
        assert!(depth.match_path(r#"{"People"}[1]{"Height"}"#));

        let depth = Depth::new(2, Some(2));
        assert!(!depth.match_path(r#"{"People"}"#));
        assert!(depth.match_path(r#"{"People"}[0]"#));
        assert!(!depth.match_path(r#"{"People"}[0]{"Age"}"#));
        assert!(!depth.match_path(r#"{"People"}[0]{"Height"}"#));
        assert!(depth.match_path(r#"{"People"}[1]"#));
        assert!(!depth.match_path(r#"{"People"}[1]{"Age"}"#));
        assert!(!depth.match_path(r#"{"People"}[1]{"Height"}"#));
    }
}
