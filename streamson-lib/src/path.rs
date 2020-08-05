//! Streams paths with indexes from input data

use crate::error;
use std::{convert::TryFrom, fmt};

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Root,
    Key(String),
    Index(usize),
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(string) => write!(f, "{{\"{}\"}}", string),
            Self::Index(idx) => write!(f, "[{}]", idx),
            Self::Root => write!(f, ""),
        }
    }
}

/// Represents the path in a json
/// e.g. {"users"}[0]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Path {
    path: Vec<Element>,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes last path element
    pub fn pop(&mut self) {
        if self.path.len() == 1 {
            panic!();
        }
        self.path.pop().unwrap();
    }

    /// Appends path element
    pub fn push(&mut self, element: Element) {
        self.path.push(element);
    }

    /// Returns the path depth
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Returns the actual path
    pub fn get_path(&self) -> &[Element] {
        &self.path
    }
}

#[derive(Debug, PartialEq)]
enum PathState {
    ElementStart,
    Array,
    ObjectStart,
    Object(bool),
    ObjectEnd,
}

impl TryFrom<&str> for Path {
    type Error = error::Path;
    fn try_from(path_str: &str) -> Result<Self, Self::Error> {
        let mut state = PathState::ElementStart;
        let mut path = Self::new();
        path.push(Element::Root);
        let mut buffer = vec![];
        for chr in path_str.chars() {
            state = match state {
                PathState::ElementStart => match chr {
                    '[' => PathState::Array,
                    '{' => PathState::ObjectStart,
                    _ => return Err(error::Path::new(path_str)),
                },
                PathState::Array => match chr {
                    '0'..='9' => {
                        buffer.push(chr);
                        PathState::Array
                    }
                    ']' => {
                        let idx: usize = buffer.drain(..).collect::<String>().parse().unwrap();
                        path.push(Element::Index(idx));
                        PathState::ElementStart
                    }
                    _ => return Err(error::Path::new(path_str)),
                },
                PathState::ObjectStart => {
                    if chr == '"' {
                        PathState::Object(false)
                    } else {
                        return Err(error::Path::new(path_str));
                    }
                }
                PathState::Object(escaped) => {
                    if escaped {
                        buffer.push(chr);
                        PathState::Object(false)
                    } else {
                        match chr {
                            '\\' => {
                                buffer.push(chr);
                                PathState::Object(true)
                            }
                            '"' => PathState::ObjectEnd,
                            _ => {
                                buffer.push(chr);
                                PathState::Object(false)
                            }
                        }
                    }
                }
                PathState::ObjectEnd => {
                    if chr == '}' {
                        let key: String = buffer.drain(..).collect();
                        path.push(Element::Key(key));
                        PathState::ElementStart
                    } else {
                        return Err(error::Path::new(path_str));
                    }
                }
            };
        }
        if state == PathState::ElementStart {
            Ok(path)
        } else {
            Err(error::Path::new(path_str))
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for element in &self.path {
            write!(f, "{}", element)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Element, Path};
    use std::convert::TryFrom;

    #[test]
    fn test_path_from_string_empty() {
        assert!(Path::try_from("").is_ok());
    }

    #[test]
    fn test_path_from_string_array() {
        let mut path = Path::new();
        path.push(Element::Root);
        path.push(Element::Index(0));
        assert_eq!(Path::try_from("[0]").unwrap(), path);
    }

    #[test]
    fn test_path_from_string_object() {
        let mut path = Path::new();
        path.push(Element::Root);
        path.push(Element::Key(r#"my-ke\\y\" "#.into()));
        assert_eq!(Path::try_from(r#"{"my-ke\\y\" "}"#).unwrap(), path);
    }
}
