//! Emits paths with indexes from input data

use crate::error;
use std::{
    collections::{vec_deque::Drain, VecDeque},
    fmt,
    str::from_utf8,
};

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

/// Output of path processing
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
    Parse(StringState),
}

#[derive(Debug, PartialEq)]
enum StringState {
    Normal,
    Escaped,
}

#[derive(Debug)]
enum States {
    Value(Element),
    Str(StringState),
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
/// ```json
/// {
///     "People": [
///         {"Height": 180, "Age": 33},
///         {"Height": 175, "Age": 24}
///     ]
/// }
/// ```
/// should emit (index and path)
/// ```text
/// Start( 0, "")
/// Start( 1, "{\"People\"}")
/// Start( 3, "{\"People\"}[0]")
/// Start( 4, "{\"People\"}[0]{\"Height\"}")
/// End(   5, "{\"People\"}[0]{\"Height\"}")
/// Start( 6, "{\"People\"}[0]{\"Age\"}")
/// End(   7, "{\"People\"}[0]{\"Age\"}")
/// End(   8, "{\"People\"}[0]")
/// Start( 9, "{\"People\"}[1]")
/// Start(10, "{\"People\"}[1]{\"Height\"}")
/// End(  11, "{\"People\"}[1]{\"Height\"}")
/// Start(12, "{\"People\"}[1]{\"Age\"}")
/// End(  13, "{\"People\"}[1]{\"Age\"}")
/// End(  14, "{\"People\"}[1]")
/// End(  15, "{\"People\"}")
/// End(  16, "")
/// Finished
/// ```
#[derive(Debug)]
pub struct Emitter {
    /// Paths stack
    path: Vec<Element>,
    /// Paring elements stack
    states: Vec<States>,
    /// Pending buffer
    pending: VecDeque<u8>,
    /// Total index of pending buffer
    pending_idx: usize,
    /// Total index agains the first byte passed to input
    total_idx: usize,
}

impl Default for Emitter {
    fn default() -> Self {
        Self {
            path: vec![],
            states: vec![States::Value(Element::Root), States::RemoveWhitespaces],
            pending: VecDeque::new(),
            pending_idx: 0,
            total_idx: 0,
        }
    }
}

impl Emitter {
    /// Creates a new path emitter
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows current path
    fn display_path(&self) -> String {
        self.path
            .iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<String>>()
            .join("")
    }

    /// Returns currently processed byte
    ///
    /// # Returns
    /// * None - needs more data
    /// * Some(byte) - current byte
    ///
    fn peek(&mut self) -> Option<u8> {
        if self.pending.len() > self.pending_idx {
            Some(self.pending[self.pending_idx])
        } else {
            None
        }
    }

    /// Moves current curser character forward
    ///
    fn forward(&mut self) {
        if self.peek().is_some() {
            self.pending_idx += 1;
        }
    }

    /// Moves pending buffer forward (reallocates data)
    fn advance(&mut self) -> Drain<u8> {
        let to_remove = self.pending_idx;
        if self.pending_idx > 0 {
            self.total_idx += self.pending_idx;
            self.pending_idx = 0;
        }
        self.pending.drain(0..to_remove)
    }

    /// Feed emitter with data
    pub fn feed(&mut self, input: &[u8]) {
        self.pending.extend(input);
    }

    /// Moves cursor forward while characters are namespace
    fn remove_whitespaces(&mut self) -> Option<usize> {
        let mut size = 0;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_whitespace() {
                self.advance();
                return Some(size);
            }
            size += 1;
            self.forward();
        }
        None
    }

    fn process_remove_whitespace(&mut self) -> Result<Option<Output>, error::General> {
        if self.remove_whitespaces().is_none() {
            self.states.push(States::RemoveWhitespaces);
            return Ok(Some(Output::Pending));
        }
        Ok(None)
    }

    fn process_value(&mut self, element: Element) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.states.push(States::Str(StringState::Normal));
                    self.advance();
                    self.forward();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                b'0'..=b'9' => {
                    self.states.push(States::Number);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                b't' | b'f' => {
                    self.states.push(States::Bool);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                b'n' => {
                    self.states.push(States::Null);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                b'[' => {
                    self.states.push(States::Array(0));
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::Value(Element::Index(0)));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                b'{' => {
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                b']' | b'}' => {
                    // End of an array or object -> no value matched
                    Ok(None)
                }
                byte => {
                    Err(error::IncorrectInput::new(byte, self.total_idx + self.pending_idx).into())
                }
            }
        } else {
            self.states.push(States::Value(element));
            Ok(Some(Output::Pending))
        }
    }

    fn process_str(&mut self, state: StringState) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    if state == StringState::Normal {
                        self.forward();
                        self.advance();
                        let prev_path = self.display_path();
                        self.path.pop().unwrap();
                        Ok(Some(Output::End(self.total_idx, prev_path)))
                    } else {
                        self.forward();
                        self.states.push(States::Str(StringState::Normal));
                        Ok(None)
                    }
                }
                b'\\' => {
                    self.forward();
                    let new_state = match state {
                        StringState::Escaped => StringState::Normal,
                        StringState::Normal => StringState::Escaped,
                    };
                    self.states.push(States::Str(new_state));
                    Ok(None)
                }
                _ => {
                    self.forward();
                    self.states.push(States::Str(StringState::Normal));
                    Ok(None)
                }
            }
        } else {
            self.states.push(States::Str(state));
            Ok(Some(Output::Pending))
        }
    }

    fn process_number(&mut self) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || byte == b'.' {
                self.forward();
                self.states.push(States::Number);
                Ok(None)
            } else {
                self.advance();
                let prev_path = self.display_path();
                self.path.pop().unwrap();
                Ok(Some(Output::End(self.total_idx, prev_path)))
            }
        } else {
            self.states.push(States::Number);
            Ok(Some(Output::Pending))
        }
    }

    fn process_bool(&mut self) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            if byte.is_ascii_alphabetic() {
                self.forward();
                self.states.push(States::Bool);
                Ok(None)
            } else {
                self.advance();
                let prev_path = self.display_path();
                self.path.pop().unwrap();
                Ok(Some(Output::End(self.total_idx, prev_path)))
            }
        } else {
            self.states.push(States::Bool);
            Ok(Some(Output::Pending))
        }
    }

    fn process_null(&mut self) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            if byte.is_ascii_alphabetic() {
                self.forward();
                self.states.push(States::Null);
                Ok(None)
            } else {
                self.advance();
                let prev_path = self.display_path();
                self.path.pop().unwrap();
                Ok(Some(Output::End(self.total_idx, prev_path)))
            }
        } else {
            self.states.push(States::Null);
            Ok(Some(Output::Pending))
        }
    }

    fn process_array(&mut self, idx: usize) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b']' => {
                    self.forward();
                    self.advance();
                    let exported_path = self.display_path();
                    self.path.pop().unwrap();
                    Ok(Some(Output::End(self.total_idx, exported_path)))
                }
                b',' => {
                    self.forward();
                    self.states.push(States::Array(idx + 1));
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::Value(Element::Index(idx + 1)));
                    self.states.push(States::RemoveWhitespaces);
                    Ok(None)
                }
                byte => {
                    Err(error::IncorrectInput::new(byte, self.total_idx + self.pending_idx).into())
                }
            }
        } else {
            self.states.push(States::Array(idx));
            Ok(Some(Output::Pending))
        }
    }

    fn process_object(&mut self) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b'}' => {
                    self.forward();
                    self.advance();
                    let exported_path = self.display_path();
                    self.path.pop().unwrap();
                    Ok(Some(Output::End(self.total_idx, exported_path)))
                }
                b',' => {
                    self.forward();
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    Ok(None)
                }
                byte => {
                    Err(error::IncorrectInput::new(byte, self.total_idx + self.pending_idx).into())
                }
            }
        } else {
            self.states.push(States::Object);
            Ok(Some(Output::Pending))
        }
    }

    fn process_object_key(
        &mut self,
        state: ObjectKeyState,
    ) -> Result<Option<Output>, error::General> {
        match state {
            ObjectKeyState::Init => {
                if let Some(byte) = self.peek() {
                    match byte {
                        b'"' => {
                            self.advance(); // move cursor to the start
                            self.forward();
                            self.states.push(States::ObjectKey(ObjectKeyState::Parse(
                                StringState::Normal,
                            )));
                            Ok(None)
                        }
                        b'}' => Ok(None), // end has been reached to Object

                        byte => Err(error::IncorrectInput::new(
                            byte,
                            self.total_idx + self.pending_idx,
                        )
                        .into()), // keys are strings in JSON
                    }
                } else {
                    self.states.push(States::ObjectKey(state));
                    Ok(Some(Output::Pending))
                }
            }
            ObjectKeyState::Parse(string_state) => {
                if let Some(byte) = self.peek() {
                    self.forward();
                    match string_state {
                        StringState::Normal => match byte {
                            b'\"' => {
                                let idx = self.pending_idx;
                                let slice = &self.advance().collect::<Vec<u8>>()[1..idx - 1];
                                let key = from_utf8(slice)?.to_string();
                                self.states.push(States::Value(Element::Key(key)));
                                self.states.push(States::RemoveWhitespaces);
                                self.states.push(States::Colon);
                                self.states.push(States::RemoveWhitespaces);
                                Ok(None)
                            }
                            b'\\' => {
                                self.states.push(States::ObjectKey(ObjectKeyState::Parse(
                                    StringState::Escaped,
                                )));
                                Ok(None)
                            }
                            _ => {
                                self.states.push(States::ObjectKey(ObjectKeyState::Parse(
                                    StringState::Normal,
                                )));
                                Ok(None)
                            }
                        },
                        StringState::Escaped => {
                            self.states.push(States::ObjectKey(ObjectKeyState::Parse(
                                StringState::Normal,
                            )));
                            Ok(None)
                        }
                    }
                } else {
                    self.states
                        .push(States::ObjectKey(ObjectKeyState::Parse(string_state)));
                    Ok(Some(Output::Pending))
                }
            }
        }
    }

    fn process_colon(&mut self) -> Result<Option<Output>, error::General> {
        if let Some(byte) = self.peek() {
            if byte != b':' {
                return Err(
                    error::IncorrectInput::new(byte, self.total_idx + self.pending_idx).into(),
                );
            }
            self.forward();
            Ok(None)
        } else {
            self.states.push(States::Colon);
            Ok(Some(Output::Pending))
        }
    }

    /// Reads data from emitter and emits [Output](enum.Output.html) struct
    ///
    /// # Errors
    ///
    /// If invalid JSON is passed and error may be emitted.
    /// Note that validity of input JSON is not checked.
    pub fn read(&mut self) -> Result<Output, error::General> {
        while let Some(state) = self.states.pop() {
            match state {
                States::RemoveWhitespaces => {
                    if let Some(output) = self.process_remove_whitespace()? {
                        return Ok(output);
                    }
                }
                States::Value(element) => {
                    if let Some(output) = self.process_value(element)? {
                        return Ok(output);
                    }
                }
                States::Str(state) => {
                    if let Some(output) = self.process_str(state)? {
                        return Ok(output);
                    }
                }
                States::Number => {
                    if let Some(output) = self.process_number()? {
                        return Ok(output);
                    }
                }
                States::Bool => {
                    if let Some(output) = self.process_bool()? {
                        return Ok(output);
                    }
                }
                States::Null => {
                    if let Some(output) = self.process_null()? {
                        return Ok(output);
                    }
                }
                States::Array(idx) => {
                    if let Some(output) = self.process_array(idx)? {
                        return Ok(output);
                    }
                }
                States::Object => {
                    if let Some(output) = self.process_object()? {
                        return Ok(output);
                    }
                }
                States::ObjectKey(state) => {
                    if let Some(output) = self.process_object_key(state)? {
                        return Ok(output);
                    }
                }
                States::Colon => {
                    if let Some(output) = self.process_colon()? {
                        return Ok(output);
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
        emitter.feed(br#"  "test string \" \\\" [ ] {} , :\\""#);
        assert_eq!(emitter.read().unwrap(), Output::Start(2, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(36, "".into()));
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
        emitter.feed(br#"{"a":"a", "b" :  true , "c": null, " \" \\\" \\": 33}"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0, "".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(5, "{\"a\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(8, "{\"a\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(17, "{\"b\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(21, "{\"b\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::Start(29, "{\"c\"}".into()));
        assert_eq!(emitter.read().unwrap(), Output::End(33, "{\"c\"}".into()));
        assert_eq!(
            emitter.read().unwrap(),
            Output::Start(50, r#"{" \" \\\" \\"}"#.into())
        );
        assert_eq!(
            emitter.read().unwrap(),
            Output::End(52, r#"{" \" \\\" \\"}"#.into())
        );
        assert_eq!(emitter.read().unwrap(), Output::End(53, "".into()));
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

    #[test]
    fn test_utf8() {
        // try to cover all utf8 character lengths
        let utf8_data: Vec<u8> = r#"[{"š𐍈€": "€š𐍈"}, "𐍈€š"]"#.to_string().into_bytes();
        for i in 0..utf8_data.len() {
            let start_data = &utf8_data[0..i];
            let end_data = &utf8_data[i..];
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
                        return e;
                    }
                    Err(_) => panic!("Error occured"),
                }
            };

            assert_eq!(get_item(), Output::Start(0, "".into()));
            assert_eq!(get_item(), Output::Start(1, "[0]".into()));
            assert_eq!(get_item(), Output::Start(15, "[0]{\"š𐍈€\"}".into()));
            assert_eq!(get_item(), Output::End(26, "[0]{\"š𐍈€\"}".into()));
            assert_eq!(get_item(), Output::End(27, "[0]".into()));
            assert_eq!(get_item(), Output::Start(29, "[1]".into()));
            assert_eq!(get_item(), Output::End(40, "[1]".into()));
            assert_eq!(get_item(), Output::End(41, "".into()));
            assert_eq!(get_item(), Output::Finished);
        }
    }
}
