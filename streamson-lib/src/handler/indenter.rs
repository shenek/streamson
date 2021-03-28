//! Handler which alters indentation of matched data
//!
//! # Example
//! ```
//! use streamson_lib::{handler, matcher, strategy::{self, Strategy}};
//! use std::sync::{Arc, Mutex};
//!
//! let handler = Arc::new(Mutex::new(handler::Indenter::new(Some(2))));
//! let mut all = strategy::All::new();
//! all.set_convert(true);
//!
//! // Set the handler for all strategy
//! all.add_handler(handler);
//!
//! for input in vec![
//!     br#"{"users": [{"password": "1234", "name": "first"}, {"#.to_vec(),
//!     br#""password": "0000", "name": "second}]}"#.to_vec(),
//! ] {
//!     for converted_data in all.process(&input).unwrap() {
//!         println!("{:?}", converted_data);
//!     }
//! }
//! ```

use super::Handler;
use crate::{
    error,
    path::{Element, Path},
    streamer::{ParsedKind, Token},
};
use std::{any::Any, str::FromStr};

/// Handler which alters indentation of matched data
#[derive(Debug)]
pub struct Indenter {
    /// How many spaces should be used for indentation
    spaces: Option<usize>,
    /// Currently processed element on each level
    stack: Option<Vec<(usize, ParsedKind)>>,
}

impl Indenter {
    /// Creates a new handler which alters indentation
    ///
    /// # Arguments
    /// * spaces - how many spaces should be used for indentation (if None - no indentation or newline should be added)
    pub fn new(spaces: Option<usize>) -> Self {
        Self {
            spaces,
            stack: None,
        }
    }

    fn write_indent_level(&self, buff: &mut Vec<u8>) {
        if let Some(stack) = self.stack.as_ref() {
            for _ in 0..(stack.len() - 1) * self.spaces.unwrap_or(0) {
                buff.push(b' ');
            }
        }
    }
}

impl FromStr for Indenter {
    type Err = error::Handler;
    fn from_str(intend_str: &str) -> Result<Self, Self::Err> {
        if intend_str.is_empty() {
            Ok(Self::new(None))
        } else {
            Ok(Self::new(Some(
                intend_str.parse::<usize>().map_err(error::Handler::new)?,
            )))
        }
    }
}

impl Handler for Indenter {
    fn start(
        &mut self,
        path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let kind = if let Token::Start(_, kind) = token {
            kind
        } else {
            unreachable![];
        };

        let mut res = vec![];
        self.stack = if let Some(mut stack) = self.stack.take() {
            stack.push((0, kind));
            // We need to add separators for nested elements
            if stack.len() > 1 {
                if stack[stack.len() - 2].0 != 0 {
                    res.push(b',');
                }
                if self.spaces.is_some() {
                    res.push(b'\n');
                }
            }
            Some(stack)
        } else {
            // stack will always have one element
            Some(vec![(0, kind)])
        };

        self.write_indent_level(&mut res);
        // stack  should have at least one element now
        let stack = self.stack.as_ref().unwrap();
        if stack.len() > 1 {
            // Write key of parent object
            if matches!(stack[stack.len() - 2].1, ParsedKind::Obj) {
                if let Element::Key(key) = &path.get_path()[path.depth() - 1] {
                    res.push(b'"');
                    res.extend(key.as_bytes());
                    res.extend(br#"":"#);
                    if self.spaces.is_some() {
                        res.push(b' ');
                    }
                } else {
                    unreachable!();
                }
            }
        }

        match kind {
            ParsedKind::Arr => {
                res.push(b'[');
            }
            ParsedKind::Obj => {
                res.push(b'{');
            }
            _ => {}
        }

        if res.is_empty() {
            Ok(None)
        } else {
            Ok(Some(res))
        }
    }

    fn feed(
        &mut self,
        data: &[u8],
        _matcher_idx: usize,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let mut result = vec![];
        if let Some(stack) = self.stack.as_ref() {
            if let Some((_, kind)) = stack.last() {
                match kind {
                    ParsedKind::Obj | ParsedKind::Arr => {}
                    _ => {
                        result.extend(data.to_vec());
                    }
                }
            }
        }
        Ok(Some(result))
    }

    fn end(
        &mut self,
        _path: &Path,
        _matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let kind = if let Token::End(_, kind) = token {
            kind
        } else {
            unreachable![];
        };

        let mut res = vec![];
        if let Some(stack) = self.stack.as_ref() {
            match kind {
                ParsedKind::Arr => {
                    if stack.last().unwrap().0 != 0 && self.spaces.is_some() {
                        res.push(b'\n');
                        self.write_indent_level(&mut res);
                    }
                    res.push(b']');
                }
                ParsedKind::Obj => {
                    if stack.last().unwrap().0 != 0 && self.spaces.is_some() {
                        res.push(b'\n');
                        self.write_indent_level(&mut res);
                    }
                    res.push(b'}');
                }
                _ => {}
            };
        }

        if let Some(stack) = self.stack.as_mut() {
            // remove item from stack and increase parent count
            stack.pop();
            // Increase count
            if let Some((idx, _)) = stack.last_mut() {
                *idx += 1;
            }

            // finish newline
            if stack.is_empty() && self.spaces.is_some() {
                res.push(b'\n');
                self.stack = None;
            }
        }

        if res.is_empty() {
            Ok(None)
        } else {
            Ok(Some(res))
        }
    }

    fn is_converter(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Indenter;
    use crate::strategy::{All, OutputConverter, Strategy};
    use rstest::*;
    use std::sync::{Arc, Mutex};

    fn make_all_with_spaces(level: Option<usize>) -> All {
        let mut all = All::new();
        all.set_convert(true);
        all.add_handler(Arc::new(Mutex::new(Indenter::new(level))));
        all
    }

    #[rstest(
        spaces,
        input,
        output,
        case::null_none(None, b"null", b"null"),
        case::null_0(Some(0), b"null", b"null\n"),
        case::null_2(Some(2), b"null", b"null\n"),
        case::obj_none(None, b"{}", b"{}"),
        case::obj_0(Some(0), b"{}", b"{}\n"),
        case::obj_2(Some(2), b"{}", b"{}\n"),
        case::arr_none(None, b"[]", b"[]"),
        case::arr_0(Some(0), b"[]", b"[]\n"),
        case::arr_2(Some(2), b"[]", b"[]\n"),
        case::str_none(None, br#""str""#, br#""str""#),
        case::str_0(Some(0), br#""str""#, b"\"str\"\n"),
        case::str_2(Some(2), br#""str""#, b"\"str\"\n"),
        before => [b"\n\n", b"\n", b" ", b""],
        after => [b"\n\n", b"\n", b" "]
    )]
    fn leafs(spaces: Option<usize>, input: &[u8], output: &[u8], before: &[u8], after: &[u8]) {
        let mut all = make_all_with_spaces(spaces);
        let mut final_input = vec![];
        final_input.extend(before);
        final_input.extend(input);
        final_input.extend(after);
        let result = OutputConverter::new().convert(&all.process(&final_input).unwrap());

        assert_eq!(result.len(), 1);
        assert_eq!((None, output.to_vec()), result[0]);
    }

    #[test]
    fn flat_array() {
        let input = b" [ \n 3 \n , null,true,\n false, \"10\"\n]".to_vec();

        // No indentation or spaces
        let mut all = make_all_with_spaces(None);
        assert_eq!(
            br#"[3,null,true,false,"10"]"#.to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // No indentation
        let mut all = make_all_with_spaces(Some(0));
        assert_eq!(
            b"[\n3,\nnull,\ntrue,\nfalse,\n\"10\"\n]\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // 2 indentation
        let mut all = make_all_with_spaces(Some(2));
        assert_eq!(
            b"[\n  3,\n  null,\n  true,\n  false,\n  \"10\"\n]\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );
    }

    #[test]
    fn nested_array() {
        let input = b" [ \n [3] \n , [],null,[[]], \"10\"\n,[[[]]]]".to_vec();

        // No indentation or spaces
        let mut all = make_all_with_spaces(None);
        assert_eq!(
            br#"[[3],[],null,[[]],"10",[[[]]]]"#.to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // No indentation
        let mut all = make_all_with_spaces(Some(0));
        assert_eq!(
            b"[\n[\n3\n],\n[],\nnull,\n[\n[]\n],\n\"10\",\n[\n[\n[]\n]\n]\n]\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // 2 indentation
        let mut all = make_all_with_spaces(Some(2));
        assert_eq!(
            b"[\n  [\n    3\n  ],\n  [],\n  null,\n  [\n    []\n  ],\n  \"10\",\n  [\n    [\n      []\n    ]\n  ]\n]\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );
    }

    #[test]
    fn flat_object() {
        let input =
            b" { \n \"1\" \n: 1 , \"2\":\"2\",   \"3\": null\n, \"4\":\n\nfalse\n\n\n}".to_vec();

        // No indentation or spaces
        let mut all = make_all_with_spaces(None);
        assert_eq!(
            br#"{"1":1,"2":"2","3":null,"4":false}"#.to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // No indentation
        let mut all = make_all_with_spaces(Some(0));
        assert_eq!(
            b"{\n\"1\": 1,\n\"2\": \"2\",\n\"3\": null,\n\"4\": false\n}\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // 2 indentation
        let mut all = make_all_with_spaces(Some(2));
        assert_eq!(
            b"{\n  \"1\": 1,\n  \"2\": \"2\",\n  \"3\": null,\n  \"4\": false\n}\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );
    }

    #[test]
    fn nested_object() {
        let input =
            b" { \n \"1\" \n: {} , \"2\":{\"2a\": {}},   \"3\": null\n, \"4\":\n\n{\"4a\": {\"4aa\": {}}}\n\n\n}".to_vec();

        // No indentation or spaces
        let mut all = make_all_with_spaces(None);
        assert_eq!(
            br#"{"1":{},"2":{"2a":{}},"3":null,"4":{"4a":{"4aa":{}}}}"#.to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // No indentation
        let mut all = make_all_with_spaces(Some(0));
        assert_eq!(
            b"{\n\"1\": {},\n\"2\": {\n\"2a\": {}\n},\n\"3\": null,\n\"4\": {\n\"4a\": {\n\"4aa\": {}\n}\n}\n}\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // 2 indentation
        let mut all = make_all_with_spaces(Some(2));
        assert_eq!(
            b"{\n  \"1\": {},\n  \"2\": {\n    \"2a\": {}\n  },\n  \"3\": null,\n  \"4\": {\n    \"4a\": {\n      \"4aa\": {}\n    }\n  }\n}\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );
    }

    #[test]
    fn complex() {
        let input =
            b" { \n \"1\" \n: [] , \"2\":{\"2a\": []},   \"3\": null\n, \"4\":\n\n[ {\"4aa\": {}}]\n\n\n}".to_vec();

        // No indentation or spaces
        let mut all = make_all_with_spaces(None);
        assert_eq!(
            br#"{"1":[],"2":{"2a":[]},"3":null,"4":[{"4aa":{}}]}"#.to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // No indentation
        let mut all = make_all_with_spaces(Some(0));
        assert_eq!(
            b"{\n\"1\": [],\n\"2\": {\n\"2a\": []\n},\n\"3\": null,\n\"4\": [\n{\n\"4aa\": {}\n}\n]\n}\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );

        // 2 indentation
        let mut all = make_all_with_spaces(Some(2));
        assert_eq!(
            b"{\n  \"1\": [],\n  \"2\": {\n    \"2a\": []\n  },\n  \"3\": null,\n  \"4\": [\n    {\n      \"4aa\": {}\n    }\n  ]\n}\n".to_vec(),
            OutputConverter::new().convert(&all.process(&input).unwrap())[0].1
        );
    }
}
