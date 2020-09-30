//! Simple path matcher

use super::MatchMaker;
use crate::{
    error,
    path::{Element, Path},
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
    fn match_path(&self, path: &Path) -> bool {
        if path.depth() != self.path.len() {
            return false;
        }

        for (element, selement) in path.get_path().iter().zip(self.path.iter()) {
            match selement {
                SimplePathElement::Key(Some(key)) => match element {
                    Element::Key(k) if k == key => {}
                    _ => return false,
                },
                SimplePathElement::Key(None) => match element {
                    Element::Key(_) => {}
                    _ => return false,
                },
                SimplePathElement::Index(idx_matches) => match element {
                    Element::Index(idx) => {
                        //  if all are not matching return false
                        if !idx_matches.0.is_empty()
                            && idx_matches.0.iter().all(|idx_match| !match idx_match {
                                (Some(start), Some(end)) => (start <= idx) && (idx < end),
                                (None, Some(end)) => idx < end,
                                (Some(start), None) => start <= idx,
                                (None, None) => unreachable!(),
                            })
                        {
                            return false;
                        }
                    }
                    _ => return false,
                },
            }
        }
        true
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
    use crate::path::Path;
    use std::{convert::TryFrom, str::FromStr};

    #[test]
    fn exact() {
        let simple = Simple::from_str(r#"{"People"}[0]{"Height"}"#).unwrap();

        assert!(!simple.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));
    }

    #[test]
    fn wild_array() {
        let simple = Simple::from_str(r#"{"People"}[]{"Height"}"#).unwrap();

        assert!(!simple.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));
    }

    #[test]
    fn ranges_array() {
        let simple = Simple::from_str(r#"{"People"}[3,4-5,5,-3,6-]{"Height"}"#).unwrap();

        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[2]{"Height"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[3]{"Height"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[4]{"Height"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[5]{"Height"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[6]{"Height"}"#).unwrap()));
    }

    #[test]
    fn wild_object() {
        let simple = Simple::from_str(r#"{"People"}[0]{}"#).unwrap();

        assert!(!simple.match_path(&Path::try_from(r#"{"People"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"Age"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"Height"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]{"Age"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[1]{"Height"}"#).unwrap()));
    }

    #[test]
    fn object_escapes() {
        let simple = Simple::from_str(r#"{"People"}[0]{"\""}"#).unwrap();
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"\""}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]{""}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]{"\"x"}"#).unwrap()));
        assert!(!simple.match_path(&Path::try_from(r#"{"People"}[0]{"y\""}"#).unwrap()));
    }

    #[test]
    fn wild_object_escapes() {
        let simple = Simple::from_str(r#"{"People"}[0]{}"#).unwrap();
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"O\"ll"}"#).unwrap()));
        assert!(simple.match_path(&Path::try_from(r#"{"People"}[0]{"O\\\"ll"}"#).unwrap()));
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
}
