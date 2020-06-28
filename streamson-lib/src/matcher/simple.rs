//! Simple path matcher

use super::MatchMaker;
use crate::error;
use std::str::FromStr;

/// Based on orignal path format {"People"}[0]{"Height"}
///
/// It matches {"People"}[0]{"Height"} - height of the first person
/// It matches {"People"}[]{"Height"} - matches the height of all people
/// It matches {"People"}[0]{} - matches all attributes of the first person
#[derive(Default, Debug, Clone)]
pub struct Simple {
    path_expr: String,
}

#[derive(Debug, PartialEq)]
enum SimpleMatcherStates {
    Normal,
    Array,
    ArrayCmp,
    ArrayWild,
    Object,
    ObjectWildStart,
    ObjectWild(bool),
    ObjectWildEnd,
    ObjectCmpStart,
    ObjectCmp(bool),
    ObjectCmpEnd,
}

impl MatchMaker for Simple {
    fn match_path(&self, path: &str) -> bool {
        let mut str_iter = path.chars();
        let mut expr_iter = self.path_expr.chars();
        let mut state = SimpleMatcherStates::Normal;
        loop {
            match state {
                SimpleMatcherStates::Normal => {
                    let (expr_opt, str_opt) = (expr_iter.next(), str_iter.next());
                    if expr_opt.is_none() || str_opt.is_none() {
                        return expr_opt == str_opt;
                    }
                    if expr_opt != str_opt {
                        return false;
                    }
                    if expr_opt == Some('[') {
                        state = SimpleMatcherStates::Array;
                    }
                    if expr_opt == Some('{') {
                        state = SimpleMatcherStates::Object;
                    }
                }
                SimpleMatcherStates::Array => {
                    let (expr_opt, str_opt) = (expr_iter.next(), str_iter.next());
                    if let (Some(expr_chr), Some(str_chr)) = (expr_opt, str_opt) {
                        if expr_chr == ']' {
                            if str_chr.is_numeric() {
                                state = SimpleMatcherStates::ArrayWild;
                            } else {
                                return false;
                            }
                        } else {
                            if str_chr != expr_chr {
                                return false;
                            }
                            if !str_chr.is_numeric() {
                                return false;
                            }
                            state = SimpleMatcherStates::ArrayCmp;
                        }
                    } else {
                        return false;
                    }
                }
                SimpleMatcherStates::ArrayCmp => {
                    let (expr_opt, str_opt) = (expr_iter.next(), str_iter.next());
                    if expr_opt.is_none() || str_opt.is_none() {
                        return false;
                    }
                    if expr_opt == str_opt {
                        if expr_opt == Some(']') {
                            state = SimpleMatcherStates::Normal;
                        } else if !expr_opt.unwrap().is_numeric() {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                SimpleMatcherStates::ArrayWild => {
                    if let Some(opt_chr) = str_iter.next() {
                        if !opt_chr.is_numeric() {
                            if opt_chr == ']' {
                                state = SimpleMatcherStates::Normal;
                                continue;
                            }
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                SimpleMatcherStates::Object => {
                    if let Some(expr_chr) = expr_iter.next() {
                        state = match expr_chr {
                            '}' => SimpleMatcherStates::ObjectWildStart,
                            '"' => SimpleMatcherStates::ObjectCmpStart,
                            _ => return false,
                        };
                    } else {
                        return false;
                    }
                }
                SimpleMatcherStates::ObjectWildStart => {
                    if let Some(opt_chr) = str_iter.next() {
                        if opt_chr == '"' {
                            state = SimpleMatcherStates::ObjectWild(false);
                            continue;
                        }
                    }
                    return false;
                }
                SimpleMatcherStates::ObjectCmpStart => {
                    if let Some(chr) = str_iter.next() {
                        if chr == '"' {
                            state = SimpleMatcherStates::ObjectCmp(false);
                            continue;
                        }
                    }
                    return false;
                }
                SimpleMatcherStates::ObjectCmp(escaped) => {
                    let (expr_opt, str_opt) = (expr_iter.next(), str_iter.next());
                    if expr_opt.is_none() || str_opt.is_none() {
                        return expr_opt == str_opt;
                    }
                    if expr_opt == str_opt {
                        if expr_opt == Some('"') && !escaped {
                            state = SimpleMatcherStates::ObjectCmpEnd;
                            continue;
                        }
                        state = if expr_opt == Some('\\')
                            && state == SimpleMatcherStates::ObjectCmp(false)
                        {
                            SimpleMatcherStates::ObjectCmp(true)
                        } else {
                            SimpleMatcherStates::ObjectCmp(false)
                        };
                        continue;
                    }
                    return false;
                }
                SimpleMatcherStates::ObjectWild(escaped) => {
                    if let Some(chr) = str_iter.next() {
                        if chr == '"' && !escaped {
                            state = SimpleMatcherStates::ObjectWildEnd;
                            continue;
                        }
                        state = if chr == '\\' && !escaped {
                            SimpleMatcherStates::ObjectWild(true)
                        } else {
                            SimpleMatcherStates::ObjectWild(false)
                        };
                        continue;
                    }
                    return false;
                }
                SimpleMatcherStates::ObjectCmpEnd => {
                    let (expr_opt, str_opt) = (expr_iter.next(), str_iter.next());
                    if expr_opt == Some('}') && str_opt == Some('}') {
                        state = SimpleMatcherStates::Normal;
                        continue;
                    }
                    return false;
                }
                SimpleMatcherStates::ObjectWildEnd => {
                    if let Some(chr) = str_iter.next() {
                        if chr == '}' {
                            state = SimpleMatcherStates::Normal;
                            continue;
                        }
                    }
                    return false;
                }
            }
        }
    }
}

impl FromStr for Simple {
    type Err = error::Matcher;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Simple::is_valid(s)?;
        Ok(Self {
            path_expr: s.into(),
        })
    }
}

impl Simple {
    /// Creates new simple matcher
    ///
    /// # Arguments
    /// * `path_expr` - path expression (e.g. `{"users"}[0]{"addresses"}{}`)
    pub fn new<T>(path_expr: T) -> Self
    where
        T: ToString,
    {
        Self {
            path_expr: path_expr.to_string(),
        }
    }

    /// Check whether the path is valid
    fn is_valid(path: &str) -> Result<(), error::Matcher> {
        let mut state = SimpleMatcherStates::Normal;
        for chr in path.chars() {
            state = match state {
                SimpleMatcherStates::Normal => match chr {
                    '[' => SimpleMatcherStates::Array,
                    '{' => SimpleMatcherStates::Object,
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::Array => match chr {
                    ']' => SimpleMatcherStates::Normal,
                    '0'..='9' => SimpleMatcherStates::ArrayCmp,
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::ArrayCmp => match chr {
                    ']' => SimpleMatcherStates::Normal,
                    '0'..='9' => SimpleMatcherStates::ArrayCmp,
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::Object => match chr {
                    '}' => SimpleMatcherStates::Normal,
                    '"' => SimpleMatcherStates::ObjectCmp(false),
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                SimpleMatcherStates::ObjectCmp(false) => match chr {
                    '"' => SimpleMatcherStates::ObjectCmpEnd,
                    '\\' => SimpleMatcherStates::ObjectCmp(true),
                    _ => SimpleMatcherStates::ObjectCmp(false),
                },
                SimpleMatcherStates::ObjectCmp(true) => SimpleMatcherStates::ObjectCmp(false),
                SimpleMatcherStates::ObjectCmpEnd => match chr {
                    '}' => SimpleMatcherStates::Normal,
                    _ => {
                        return Err(error::Matcher::Parse(path.to_string()));
                    }
                },
                _ => unreachable!(),
            }
        }
        if state == SimpleMatcherStates::Normal {
            Ok(())
        } else {
            Err(error::Matcher::Parse(path.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MatchMaker, Simple};
    use std::str::FromStr;

    #[test]
    fn simple_exact() {
        let simple = Simple::from_str(r#"{"People"}[0]{"Height"}"#).unwrap();

        assert!(!simple.match_path(r#"{"People"}"#));
        assert!(!simple.match_path(r#"{"People"}[0]"#));
        assert!(!simple.match_path(r#"{"People"}[0]{"Age"}"#));
        assert!(simple.match_path(r#"{"People"}[0]{"Height"}"#));
        assert!(!simple.match_path(r#"{"People"}[1]"#));
        assert!(!simple.match_path(r#"{"People"}[1]{"Age"}"#));
        assert!(!simple.match_path(r#"{"People"}[1]{"Height"}"#));
    }

    #[test]
    fn simple_wild_array() {
        let simple = Simple::from_str(r#"{"People"}[]{"Height"}"#).unwrap();

        assert!(!simple.match_path(r#"{"People"}"#));
        assert!(!simple.match_path(r#"{"People"}[0]"#));
        assert!(!simple.match_path(r#"{"People"}[0]{"Age"}"#));
        assert!(simple.match_path(r#"{"People"}[0]{"Height"}"#));
        assert!(!simple.match_path(r#"{"People"}[1]"#));
        assert!(!simple.match_path(r#"{"People"}[1]{"Age"}"#));
        assert!(simple.match_path(r#"{"People"}[1]{"Height"}"#));
    }

    #[test]
    fn simple_wild_object() {
        let simple = Simple::from_str(r#"{"People"}[0]{}"#).unwrap();

        assert!(!simple.match_path(r#"{"People"}"#));
        assert!(!simple.match_path(r#"{"People"}[0]"#));
        assert!(simple.match_path(r#"{"People"}[0]{"Age"}"#));
        assert!(simple.match_path(r#"{"People"}[0]{"Height"}"#));
        assert!(!simple.match_path(r#"{"People"}[1]"#));
        assert!(!simple.match_path(r#"{"People"}[1]{"Age"}"#));
        assert!(!simple.match_path(r#"{"People"}[1]{"Height"}"#));
    }

    #[test]
    fn simple_object_escapes() {
        let simple = Simple::from_str(r#"{"People"}[0]{"\""}"#).unwrap();
        assert!(simple.match_path(r#"{"People"}[0]{"\""}"#));
        assert!(!simple.match_path(r#"{"People"}[0]{""}"#));
        assert!(!simple.match_path(r#"{"People"}[0]{"\"x"}"#));
        assert!(!simple.match_path(r#"{"People"}[0]{"y\""}"#));
    }

    #[test]
    fn simple_wild_object_escapes() {
        let simple = Simple::from_str(r#"{"People"}[0]{}"#).unwrap();
        assert!(simple.match_path(r#"{"People"}[0]{"O\"ll"}"#));
        assert!(simple.match_path(r#"{"People"}[0]{"O\\\"ll"}"#));
    }

    #[test]
    fn simple_parse() {
        assert!(Simple::from_str(r#""#).is_ok());
        assert!(Simple::from_str(r#"{}"#).is_ok());
        assert!(Simple::from_str(r#"{}[3]"#).is_ok());
        assert!(Simple::from_str(r#"{"xx"}[]"#).is_ok());
        assert!(Simple::from_str(r#"{}[]"#).is_ok());
        assert!(Simple::from_str(r#"{"š𐍈€"}"#).is_ok());
        assert!(Simple::from_str(r#"{"\""}"#).is_ok());
    }

    #[test]
    fn simple_parse_error() {
        assert!(Simple::from_str(r#"{"People""#).is_err());
        assert!(Simple::from_str(r#"[}"#).is_err());
        assert!(Simple::from_str(r#"{"People}"#).is_err());
        assert!(Simple::from_str(r#"{"š𐍈€""#).is_err());
    }
}
