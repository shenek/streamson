use crate::error::GenericError;
use bytes::{Buf, BytesMut};
use std::{fmt, str::from_utf8};

#[derive(Debug)]
enum Element {
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

#[derive(Debug, PartialEq)]
pub enum Output {
    /// Path begings here
    Start(usize, String),
    /// Path ends here
    End(usize, String),
    /// Needs more data
    Pending,
    /// No data left in json
    Finished,
}

#[derive(Debug)]
enum ObjectKeyState {
    Init,
    Parse,
}

#[derive(Debug)]
enum States {
    Value(Element),
    Str,
    Number,
    Bool,
    Null,
    Array(usize),
    Object,
    ObjectKey(ObjectKeyState),
    Colon,
    RemoveWhitespaces,
}

/// Reads parts of UTF-8 json input and emits paths
/// e.g. reading of
/// {
///     "People": [
///         {"Height": 180, "Age": 33},
///         {"Height": 175, "Age": 24}
///     ]
/// }
///
/// should emit (index and path)
/// X1, {"People"}
/// X2, {"People"}[0]
/// X3, {"People"}[0]{"Height"}
/// X4, {"People"}[0]{"Age"}
/// X5, {"People"}[1]
/// X6, {"People"}[1]{"Height"}
/// X7, {"People"}[1]{"Age"}
#[derive(Debug)]
pub struct Emitter {
    path: Vec<Element>,
    states: Vec<States>,
    pending: BytesMut,
    pending_idx: usize,
    total_idx: usize,
}

impl Default for Emitter {
    fn default() -> Self {
        Self {
            path: vec![],
            states: vec![States::Value(Element::Root), States::RemoveWhitespaces],
            pending: BytesMut::default(),
            pending_idx: 0,
            total_idx: 0,
        }
    }
}

impl Emitter {
    pub fn new() -> Self {
        Self::default()
    }

    fn display_path(&self) -> String {
        self.path
            .iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<String>>()
            .join("")
    }

    fn peek(&self) -> Option<char> {
        if self.pending.len() > self.pending_idx {
            let chr: char = self.pending[self.pending_idx].into();
            Some(chr)
        } else {
            None
        }
    }

    fn forward(&mut self) {
        self.pending_idx += 1;
    }

    fn advance(&mut self) {
        if self.pending_idx > 0 {
            self.pending.advance(self.pending_idx);
            self.total_idx += self.pending_idx;
            self.pending_idx = 0;
        }
    }

    pub fn feed(&mut self, input: &[u8]) {
        self.pending.extend(input);
    }

    fn remove_whitespaces(&mut self) -> bool {
        while let Some(chr) = self.peek() {
            if !chr.is_ascii_whitespace() {
                self.advance();
                return true;
            }
            self.forward();
        }
        false
    }

    pub fn read(&mut self) -> Result<Output, GenericError> {
        while let Some(state) = self.states.pop() {
            match state {
                States::RemoveWhitespaces => {
                    if !self.remove_whitespaces() {
                        self.states.push(States::RemoveWhitespaces);
                        return Ok(Output::Pending);
                    }
                }
                States::Value(element) => {
                    if let Some(chr) = self.peek() {
                        match chr {
                            '"' => {
                                self.states.push(States::Str);
                                self.advance();
                                self.forward();
                                self.path.push(element);
                                return Ok(Output::Start(self.total_idx, self.display_path()));
                            }
                            '0'..='9' => {
                                self.states.push(States::Number);
                                self.advance();
                                self.path.push(element);
                                return Ok(Output::Start(self.total_idx, self.display_path()));
                            }
                            't' | 'f' => {
                                self.states.push(States::Bool);
                                self.advance();
                                self.path.push(element);
                                return Ok(Output::Start(self.total_idx, self.display_path()));
                            }
                            'n' => {
                                self.states.push(States::Null);
                                self.advance();
                                self.path.push(element);
                                return Ok(Output::Start(self.total_idx, self.display_path()));
                            }
                            '[' => {
                                self.states.push(States::Array(0));
                                self.states.push(States::RemoveWhitespaces);
                                self.states.push(States::Value(Element::Index(0)));
                                self.states.push(States::RemoveWhitespaces);
                                self.advance();
                                self.forward();
                                self.path.push(element);
                                return Ok(Output::Start(self.total_idx, self.display_path()));
                            }
                            '{' => {
                                self.states.push(States::Object);
                                self.states.push(States::RemoveWhitespaces);
                                self.states.push(States::ObjectKey(ObjectKeyState::Init));
                                self.states.push(States::RemoveWhitespaces);
                                self.advance();
                                self.forward();
                                self.path.push(element);
                                return Ok(Output::Start(self.total_idx, self.display_path()));
                            }
                            ']' | '}' => {
                                // End of an array or object -> no value matched
                            }
                            _ => return Err(GenericError),
                        }
                    } else {
                        self.states.push(States::Value(element));
                        return Ok(Output::Pending);
                    }
                }
                States::Str => {
                    if let Some(chr) = self.peek() {
                        if chr != '"' {
                            self.forward();
                            self.states.push(States::Str);
                        } else {
                            self.forward();
                            self.advance();
                            let prev_path = self.display_path();
                            self.path.pop().ok_or_else(|| GenericError)?;
                            return Ok(Output::End(self.total_idx, prev_path));
                        }
                    } else {
                        self.states.push(States::Str);
                        return Ok(Output::Pending);
                    }
                }
                States::Number => {
                    if let Some(chr) = self.peek() {
                        if chr.is_digit(10) || chr == '.' {
                            self.forward();
                            self.states.push(States::Number);
                        } else {
                            self.advance();
                            let prev_path = self.display_path();
                            self.path.pop().ok_or_else(|| GenericError)?;
                            return Ok(Output::End(self.total_idx, prev_path));
                        }
                    } else {
                        self.states.push(States::Number);
                        return Ok(Output::Pending);
                    }
                }
                States::Bool => {
                    if let Some(chr) = self.peek() {
                        if chr.is_alphabetic() {
                            self.forward();
                            self.states.push(States::Bool);
                        } else {
                            self.advance();
                            let prev_path = self.display_path();
                            self.path.pop().ok_or_else(|| GenericError)?;
                            return Ok(Output::End(self.total_idx, prev_path));
                        }
                    } else {
                        self.states.push(States::Bool);
                        return Ok(Output::Pending);
                    }
                }
                States::Null => {
                    if let Some(chr) = self.peek() {
                        if chr.is_alphabetic() {
                            self.forward();
                            self.states.push(States::Null);
                        } else {
                            self.advance();
                            let prev_path = self.display_path();
                            self.path.pop().ok_or_else(|| GenericError)?;
                            return Ok(Output::End(self.total_idx, prev_path));
                        }
                    } else {
                        self.states.push(States::Null);
                        return Ok(Output::Pending);
                    }
                }
                States::Array(idx) => {
                    if let Some(chr) = self.peek() {
                        match chr {
                            ']' => {
                                self.forward();
                                self.advance();
                                let exported_path = self.display_path();
                                self.path.pop().ok_or(GenericError)?;
                                return Ok(Output::End(self.total_idx, exported_path));
                            }
                            ',' => {
                                self.forward();
                                self.states.push(States::Array(idx + 1));
                                self.states.push(States::RemoveWhitespaces);
                                self.states.push(States::Value(Element::Index(idx + 1)));
                                self.states.push(States::RemoveWhitespaces);
                            }
                            _ => return Err(GenericError),
                        }
                    } else {
                        self.states.push(States::Array(idx));
                        return Ok(Output::Pending);
                    }
                }
                States::Object => {
                    if let Some(chr) = self.peek() {
                        match chr {
                            '}' => {
                                self.forward();
                                self.advance();
                                let exported_path = self.display_path();
                                self.path.pop().ok_or_else(|| GenericError)?;
                                return Ok(Output::End(self.total_idx, exported_path));
                            }
                            ',' => {
                                self.forward();
                                self.states.push(States::Object);
                                self.states.push(States::RemoveWhitespaces);
                                self.states.push(States::ObjectKey(ObjectKeyState::Init));
                                self.states.push(States::RemoveWhitespaces);
                            }
                            _ => return Err(GenericError),
                        }
                    } else {
                        self.states.push(States::Object);
                        return Ok(Output::Pending);
                    }
                }
                States::ObjectKey(state) => {
                    match state {
                        ObjectKeyState::Init => {
                            if let Some(chr) = self.peek() {
                                match chr {
                                    '"' => {
                                        self.advance(); // move cursor to the start
                                        self.forward();
                                        self.states.push(States::ObjectKey(ObjectKeyState::Parse));
                                    }
                                    '}' => {} // end has been reached to Object
                                    _ => return Err(GenericError), // keys are strings in JSON
                                }
                            } else {
                                self.states.push(States::ObjectKey(state));
                                return Ok(Output::Pending);
                            }
                        }
                        ObjectKeyState::Parse => {
                            if let Some(chr) = self.peek() {
                                self.forward();
                                if chr == '"' {
                                    let key = from_utf8(&self.pending[1..self.pending_idx - 1])
                                        .map_err(|_| GenericError)?
                                        .to_string();
                                    self.states.push(States::Value(Element::Key(key)));
                                    self.states.push(States::RemoveWhitespaces);
                                    self.states.push(States::Colon);
                                    self.states.push(States::RemoveWhitespaces);
                                } else {
                                    self.states.push(States::ObjectKey(ObjectKeyState::Parse));
                                }
                            } else {
                                self.states.push(States::ObjectKey(state));
                                return Ok(Output::Pending);
                            }
                        }
                    }
                }
                States::Colon => {
                    // Process colon
                    if let Some(chr) = self.peek() {
                        if chr != ':' {
                            return Err(GenericError);
                        }
                        self.forward();
                    } else {
                        self.states.push(States::Colon);
                        return Ok(Output::Pending);
                    }
                }
            }
        }
        Ok(Output::Finished)
    }
}

#[cfg(test)]
mod tests {
    use super::{Emitter, Output};

    #[test]
    fn test_spaces() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"  "#);
        assert_eq!(emitter.read().unwrap(), Output::Pending);
    }

    #[test]
    fn test_string() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"  "test string [ ] {} , :""#);
        assert_eq!(emitter.read().unwrap(), Output::Start(2, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(26, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);

        let mut emitter = Emitter::new();
        emitter.feed(br#"" another one " "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(15, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_number() {
        let mut emitter = Emitter::new();
        emitter.feed(br#" 3.24 "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(1, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(5, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_bool() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"  true  "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(2, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(6, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_null() {
        let mut emitter = Emitter::new();
        // TODO think of some better way to terminate the nulls/bools/numbers
        emitter.feed(br#"null"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Pending);

        let mut emitter = Emitter::new();
        emitter.feed(br#"null  "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(4, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_array() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[ null, 33, "string" ]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(2, "[0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(6, "[0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(8, "[1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(10, "[1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(12, "[2]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(20, "[2]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(22, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_array_pending() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[ null, 3"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(2, "[0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(6, "[0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(8, "[1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Pending);
        emitter.feed(br#"3,"#);
        assert_eq!(emitter.read().unwrap(), Output::End(10, "[1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Pending);
        emitter.feed(br#" "string" ]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(12, "[2]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(20, "[2]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(22, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_empty_array() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(2, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_array_in_array() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[ [], 33, ["string" , 44], [  ]]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(2, "[0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(4, "[0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(6, "[1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(8, "[1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(10, "[2]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(11, "[2][0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(19, "[2][0]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(22, "[2][1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(24, "[2][1]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(25, "[2]".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(27, "[3]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(31, "[3]".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(32, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_object() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"{"a":"a", "b" :  true , "c": null}"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(5, "{\"a\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(8, "{\"a\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(17, "{\"b\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(21, "{\"b\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(29, "{\"c\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(33, "{\"c\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(34, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_empty_object() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"{}"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(2, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_object_in_object() {
        let mut emitter = Emitter::new();
        emitter.feed(br#" {"u": {}, "j": {"x": {  }, "y": 10}} "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(1, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(7, "{\"u\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(9, "{\"u\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(16, "{\"j\"}".into()));
        assert_eq!(
            emitter.read().unwrap(),
            Output::Start(22, "{\"j\"}{\"x\"}".into())
        );
        assert_eq!(
            emitter.read().unwrap(),
            Output::End(26, "{\"j\"}{\"x\"}".into())
        );
        assert_eq!(
            emitter.read().unwrap(),
            Output::Start(33, "{\"j\"}{\"y\"}".into())
        );
        assert_eq!(
            emitter.read().unwrap(),
            Output::End(35, "{\"j\"}{\"y\"}".into())
        );
        assert_eq!(emitter.read().unwrap(), Output::End(36, "{\"j\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(37, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_complex_with_pending() {
        const COMPLEX_DATA: &[u8] = br#" [{"aha y": {}, "j": {"x": [{  }, [ {}, null ]], "y" : 10}}, null, 43, [ {"a": false} ] ]"#;

        // Split complex data into parts
        for i in 0..COMPLEX_DATA.len() {
            let start_data = &COMPLEX_DATA[0..i];
            let end_data = &COMPLEX_DATA[i..];
            let mut emitter = Emitter::new();

            // feed the first part
            emitter.feed(start_data);

            // Gets next item and feed the rest of the data when pending
            let mut get_item = || loop {
                match emitter.read() {
                    Ok(Output::Pending) => {
                        emitter.feed(end_data);
                        continue;
                    }
                    Ok(e) => {
                        dbg!(&e);
                        return e;
                    }
                    Err(_) => panic!("Error occured"),
                }
            };

            assert_eq!(get_item(), Output::Start(1, "".into()));
            assert_eq!(get_item(), Output::Start(2, "[0]".into()));
            assert_eq!(get_item(), Output::Start(12, "[0]{\"aha y\"}".into()));
            assert_eq!(get_item(), Output::End(14, "[0]{\"aha y\"}".into()));
            assert_eq!(get_item(), Output::Start(21, "[0]{\"j\"}".into()));
            assert_eq!(get_item(), Output::Start(27, "[0]{\"j\"}{\"x\"}".into()));
            assert_eq!(get_item(), Output::Start(28, "[0]{\"j\"}{\"x\"}[0]".into()));
            assert_eq!(get_item(), Output::End(32, "[0]{\"j\"}{\"x\"}[0]".into()));
            assert_eq!(get_item(), Output::Start(34, "[0]{\"j\"}{\"x\"}[1]".into()));
            assert_eq!(
                get_item(),
                Output::Start(36, "[0]{\"j\"}{\"x\"}[1][0]".into())
            );
            assert_eq!(
                get_item(),
                Output::End(38, "[0]{\"j\"}{\"x\"}[1][0]".into())
            );
            assert_eq!(
                get_item(),
                Output::Start(40, "[0]{\"j\"}{\"x\"}[1][1]".into())
            );
            assert_eq!(
                get_item(),
                Output::End(44, "[0]{\"j\"}{\"x\"}[1][1]".into())
            );
            assert_eq!(get_item(), Output::End(46, "[0]{\"j\"}{\"x\"}[1]".into()));
            assert_eq!(get_item(), Output::End(47, "[0]{\"j\"}{\"x\"}".into()));
            assert_eq!(get_item(), Output::Start(55, "[0]{\"j\"}{\"y\"}".into()));
            assert_eq!(get_item(), Output::End(57, "[0]{\"j\"}{\"y\"}".into()));
            assert_eq!(get_item(), Output::End(58, "[0]{\"j\"}".into()));
            assert_eq!(get_item(), Output::End(59, "[0]".into()));
            assert_eq!(get_item(), Output::Start(61, "[1]".into()));
            assert_eq!(get_item(), Output::End(65, "[1]".into()));
            assert_eq!(get_item(), Output::Start(67, "[2]".into()));
            assert_eq!(get_item(), Output::End(69, "[2]".into()));
            assert_eq!(get_item(), Output::Start(71, "[3]".into()));
            assert_eq!(get_item(), Output::Start(73, "[3][0]".into()));
            assert_eq!(get_item(), Output::Start(79, "[3][0]{\"a\"}".into()));
            assert_eq!(get_item(), Output::End(84, "[3][0]{\"a\"}".into()));
            assert_eq!(get_item(), Output::End(85, "[3][0]".into()));
            assert_eq!(get_item(), Output::End(87, "[3]".into()));
            assert_eq!(get_item(), Output::End(89, "".into()));
            assert_eq!(get_item(), Output::Finished);
        }
    }
}
