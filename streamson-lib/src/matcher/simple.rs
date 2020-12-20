//! Simple path matcher

use super::MatchMaker;
use crate::{
    error,
    path::{Element, Path},
    streamer::ParsedKind,
};
use std::str::FromStr;

/// StringMatch to match array elements
type StringMatch = Option<String>;

/// IndexMatch to match array elements
#[derive(Debug, Clone, PartialEq)]
struct IndexMatch(Vec<(Option<usize>, Option<usize>)>);

impl FromStr for IndexMatch {
    type Err = error::Matcher;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let splitted = path.split(',');
        let mut result = vec![];

        for item_str in splitted {
            let inner_splitted: Vec<_> = item_str.split('-').collect();
            match inner_splitted.len() {
                1 => {
                    let index: usize = inner_splitted[0]
                        .parse()
                        .map_err(|_| error::Matcher::Parse(inner_splitted[0].to_string()))?;
                    result.push((Some(index), Some(index + 1)));
                }
                2 => {
                    let start_opt: Option<usize> =
                        if inner_splitted[0].is_empty() {
                            None
                        } else {
                            Some(inner_splitted[0].parse().map_err(|_| {
                                error::Matcher::Parse(inner_splitted[0].to_string())
                            })?)
                        };
                    let end_opt: Option<usize> =
                        if inner_splitted[1].is_empty() {
                            None
                        } else {
                            Some(inner_splitted[1].parse().map_err(|_| {
                                error::Matcher::Parse(inner_splitted[1].to_string())
                            })?)
                        };
                    match (start_opt, end_opt) {
                        (Some(start), Some(end)) => {
                            if start >= end {
                                return Err(error::Matcher::Parse(item_str.to_string()));
                            }
                        }
                        (None, None) => return Err(error::Matcher::Parse(item_str.to_string())),
                        _ => {}
                    }
                    result.push((start_opt, end_opt));
                }
                _ => return Err(error::Matcher::Parse(item_str.to_string())),
            }
        }

        Ok(Self(result))
    }
}

/// SimplePath path matcher
#[derive(Debug, Clone, PartialEq)]
enum SimplePathElement {
    Key(StringMatch),
    Index(IndexMatch),
    WildCardSingle,
    WildCardAny,
}

impl PartialEq<Element> for SimplePathElement {
    fn eq(&self, other: &Element) -> bool {
        match &self {
            SimplePathElement::Key(None) => other.is_key(),
            SimplePathElement::Key(Some(key)) => {
                if let Element::Key(pkey) = other {
                    key == pkey
                } else {
                    false
                }
            }
            SimplePathElement::Index(idx_matches) => {
                if let Element::Index(idx) = other {
                    if idx_matches.0.is_empty() {
                        true
                    } else {
                        idx_matches.0.iter().any(|(min_opt, max_opt)| {
                            if let Some(max) = max_opt {
                                if idx >= max {
                                    return false;
                                }
                            }
                            if let Some(min) = min_opt {
                                if idx < min {
                                    return false;
                                }
                            }
                            true
                        })
                    }
                } else {
                    false
                }
            }
            SimplePathElement::WildCardAny => true,
            SimplePathElement::WildCardSingle => true,
        }
    }
}

/// Based on orignal path format {"People"}[0]{"Height"}
///
/// It matches {"People"}[0]{"Height"} - height of the first person
/// It matches {"People"}[]{"Height"} - matches the height of all people
/// It matches {"People"}[0]{} - matches all attributes of the first person
#[derive(Default, Debug, Clone)]
pub struct Simple {
    path: Vec<SimplePathElement>,
}

#[derive(Debug, PartialEq)]
enum SimpleMatcherStates {
    ElementStart,
    Array,
    ObjectStart,
    Object(bool),
    ObjectEnd,
}

impl MatchMaker for Simple {
    fn match_path(&self, path: &Path, _kind: ParsedKind) -> bool {
        // If no AnyWildcard present and length differs
        // return false right away
        if !self
            .path
            .iter()
            .any(|e| matches!(e, SimplePathElement::WildCardAny))
            && path.depth() != self.path.len()
        {
            return false;
        }

        let path = path.get_path();

        // first is element idx, second path index
        // starting at the beginning
        let mut indexes = vec![(0, 0)];

        while !indexes.is_empty() {
            let (spath_idx, path_idx) = indexes.pop().unwrap();

            if spath_idx == self.path.len() && path_idx == path.len() {
                // all matched
                return true;
            }

            if spath_idx >= self.path.len() {
                // matcher lenght reached => fallback
                continue;
            }

            // match indexes
            match self.path[spath_idx] {
                SimplePathElement::WildCardAny => {
                    indexes.push((spath_idx + 1, path_idx)); // wildcard over
                    if path_idx < path.len() {
                        indexes.push((spath_idx, path_idx + 1)); // wildcard matched
                    }
                }
                _ => {
                    if path_idx >= path.len() {
                        continue;
                    } else if self.path[spath_idx] == path[path_idx] {
                        indexes.push((spath_idx + 1, path_idx + 1));
                    } else {
                        continue;
                    }
                }
            }
        }

        false
    }
}

impl FromStr for Simple {
    type Err = error::Matcher;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let mut state = SimpleMatcherStates::ElementStart;
        let mut buffer = vec![];
        let mut result = vec![];

        for chr in path.chars() {
            state = match state {
                SimpleMatcherStates::ElementStart => match chr {
                    '[' => SimpleMatcherStates::Array,
                    '{' => SimpleMatcherStates::ObjectStart,
                    '?' => {
                        result.push(SimplePathElement::WildCardSingle);
                        SimpleMatcherStates::ElementStart
                    }
                    '*' => {
                        result.push(SimplePathElement::WildCardAny);
                        SimpleMatcherStates::ElementStart
                    }
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::Array => match chr {
                    ']' => {
                        let new_element = if buffer.is_empty() {
                            SimplePathElement::Index(IndexMatch(vec![]))
                        } else {
                            SimplePathElement::Index(
                                buffer
                                    .drain(..)
                                    .collect::<String>()
                                    .parse()
                                    .map_err(|_| error::Matcher::Parse(path.to_string()))?,
                            )
                        };
                        result.push(new_element);
                        SimpleMatcherStates::ElementStart
                    }
                    '0'..='9' | '-' | ',' => {
                        buffer.push(chr);
                        SimpleMatcherStates::Array
                    }
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::ObjectStart => match chr {
                    '}' => {
                        result.push(SimplePathElement::Key(None));
                        SimpleMatcherStates::ElementStart
                    }
                    '"' => SimpleMatcherStates::Object(false),
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::Object(false) => match chr {
                    '"' => SimpleMatcherStates::ObjectEnd,
                    '\\' => {
                        buffer.push(chr);
                        SimpleMatcherStates::Object(true)
                    }
                    _ => {
                        buffer.push(chr);
                        SimpleMatcherStates::Object(false)
                    }
                },
                SimpleMatcherStates::Object(true) => {
                    buffer.push(chr);
                    SimpleMatcherStates::Object(false)
                }
                SimpleMatcherStates::ObjectEnd => match chr {
                    '}' => {
                        result.push(SimplePathElement::Key(Some(buffer.drain(..).collect())));
                        SimpleMatcherStates::ElementStart
                    }
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
            }
        }
        if state == SimpleMatcherStates::ElementStart {
            Ok(Self { path: result })
        } else {
            Err(error::Matcher::Parse(path.to_string()))
        }
    }
}

impl Simple {
    /// Creates new simple matcher
    ///
    /// # Arguments
    /// * `path_expr` - path expression (e.g. `{"users"}[0]{"addresses"}{}`)
    pub fn new(path_expr: &str) -> Result<Self, error::Matcher> {
        Self::from_str(path_expr)
    }
}

#[cfg(test)]
mod tests {
    use super::{MatchMaker, Simple};
    use crate::{path::Path, streamer::ParsedKind};
    use std::{convert::TryFrom, str::FromStr};

    #[test]
    fn exact() {
        let simple = Simple::from_str(r#"{"People"}[0]{"Height"}"#).unwrap();

        assert!(!simple.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn wild_array() {
        let simple = Simple::from_str(r#"{"People"}[]{"Height"}"#).unwrap();

        assert!(!simple.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn ranges_array() {
        let simple = Simple::from_str(r#"{"People"}[3,4-5,5,-3,6-]{"Height"}"#).unwrap();

        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[2]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[3]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[4]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[5]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[6]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn wild_object() {
        let simple = Simple::from_str(r#"{"People"}[0]{}"#).unwrap();

        assert!(!simple.match_path(&Path::try_from(r#"{"People"}"#).unwrap(), ParsedKind::Arr));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn object_escapes() {
        let simple = Simple::from_str(r#"{"People"}[0]{"\""}"#).unwrap();
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"\""}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]{""}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"\"x"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"y\""}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn wild_object_escapes() {
        let simple = Simple::from_str(r#"{"People"}[0]{}"#).unwrap();
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"O\"ll"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"O\\\"ll"}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn parse() {
        assert!(Simple::from_str(r#""#).is_ok());
        assert!(Simple::from_str(r#"{}"#).is_ok());
        assert!(Simple::from_str(r#"{}[3]"#).is_ok());
        assert!(Simple::from_str(r#"{"xx"}[]"#).is_ok());
        assert!(Simple::from_str(r#"{}[]"#).is_ok());
        assert!(Simple::from_str(r#"{"š𐍈€"}"#).is_ok());
        assert!(Simple::from_str(r#"{"\""}"#).is_ok());
        assert!(Simple::from_str(r#"[1,2,8,3-,-2,2-3]"#).is_ok());
        assert!(Simple::from_str(r#"?"#).is_ok());
        assert!(Simple::from_str(r#"????"#).is_ok());
        assert!(Simple::from_str(r#"?{}[1]?{"xx"}"#).is_ok());
        assert!(Simple::from_str(r#"*"#).is_ok());
        assert!(Simple::from_str(r#"****"#).is_ok());
        assert!(Simple::from_str(r#"*{}[1]**{"xx"}*"#).is_ok());
    }

    #[test]
    fn parse_error() {
        assert!(Simple::from_str(r#"{"People""#).is_err());
        assert!(Simple::from_str(r#"[}"#).is_err());
        assert!(Simple::from_str(r#"{"People}"#).is_err());
        assert!(Simple::from_str(r#"{"š𐍈€""#).is_err());
        assert!(Simple::from_str(r#"[1,2,8,3-,-2,-]"#).is_err());
        assert!(Simple::from_str(r#"[3-3]"#).is_err());
        assert!(Simple::from_str(r#"[,2,8]"#).is_err());
        assert!(Simple::from_str(r#"[2,8,]"#).is_err());
    }

    #[test]
    fn single_wild() {
        let simple = Simple::from_str(r#"?[0]{"range"}?"#).unwrap();

        assert!(simple.match_path(
            &Path::try_from(r#"[1][0]{"range"}{"from_home"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"range"}[1]"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"[0]{"range"}{"from_home"}"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[0]{"range"}"#).unwrap(),
            ParsedKind::Arr
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"{"People"}[1]{"range"}[1]"#).unwrap(),
            ParsedKind::Num
        ));
        assert!(!simple.match_path(
            &Path::try_from(r#"[1][0]{"other"}{"from_home"}"#).unwrap(),
            ParsedKind::Num
        ));
    }

    #[test]
    fn any_wild() {
        let simple = Simple::from_str(r#"*[0]*{"range"}**"#).unwrap();

        assert!(simple.match_path(&Path::try_from(r#"[0]{"range"}"#).unwrap(), ParsedKind::Obj));
        assert!(simple.match_path(
            &Path::try_from(r#"[1][0]{"range"}{"from_home"}"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"{"another"}[1][0]{"range"}{"from_home"}[2]"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"[0][2]{"range"}"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(simple.match_path(
            &Path::try_from(r#"[0]{"middle"}{"range"}"#).unwrap(),
            ParsedKind::Obj
        ));
        assert!(!simple.match_path(&Path::try_from(r#"[1]{"range"}"#).unwrap(), ParsedKind::Obj));
        assert!(!simple.match_path(&Path::try_from(r#"[0]{"other"}"#).unwrap(), ParsedKind::Obj));
    }
}
