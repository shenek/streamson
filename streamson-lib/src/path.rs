//! Emits paths with indexes from input data

use crate::error;
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
    pending: BytesMut,
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
            pending: BytesMut::default(),
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

    /// Checks which utf8 character is currently process
    fn peek(&self) -> Result<Option<char>, error::Generic> {
        let get_char = |size: usize| {
            if self.pending.len() >= self.pending_idx + size {
                let decoded = from_utf8(&self.pending[self.pending_idx..self.pending_idx + size])
                    .map_err(|_| error::Generic)?;
                Ok(decoded.chars().next()) // Should only return only a single character
            } else {
                Ok(None)
            }
        };
        if self.pending.len() > self.pending_idx {
            match self.pending[self.pending_idx] {
                0b0000_0000..=0b0111_1111 => get_char(1),
                0b1100_0000..=0b1101_1111 => get_char(2),
                0b1110_0000..=0b1110_1111 => get_char(3),
                0b1111_0000..=0b1111_0111 => get_char(4),
                _ => {
                    // input is not UTF8
                    Err(error::Generic)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Moves current curser character forward
    ///
    /// # Errors
    /// * `Err(_)` -> wrong utf-8
    /// * Ok(None) -> need more data
    /// * Ok(usize) -> read X characters (1-4)
    fn forward(&mut self) -> Result<Option<usize>, error::Generic> {
        if let Some(chr) = self.peek()? {
            let len = chr.len_utf8();
            self.pending_idx += len;
            Ok(Some(len))
        } else {
            Ok(None)
        }
    }

    /// Muves pending buffer forward (reallocates data)
    fn advance(&mut self) {
        if self.pending_idx > 0 {
            self.pending.advance(self.pending_idx);
            self.total_idx += self.pending_idx;
            self.pending_idx = 0;
        }
    }

    /// Feed emitter with data
    pub fn feed(&mut self, input: &[u8]) {
        self.pending.extend(input);
    }

    /// Moves cursor forward while characters are namespace
    fn remove_whitespaces(&mut self) -> Result<Option<usize>, error::Generic> {
        let mut size = 0;
        while let Some(chr) = self.peek()? {
            if !chr.is_ascii_whitespace() {
                self.advance();
                return Ok(Some(size));
            }
            size += self.forward()?.unwrap();
        }
        Ok(None)
    }

    fn process_remove_whitespace(&mut self) -> Result<Option<Output>, error::Generic> {
        if self.remove_whitespaces()?.is_none() {
            self.states.push(States::RemoveWhitespaces);
            return Ok(Some(Output::Pending));
        }
        Ok(None)
    }

    fn process_value(&mut self, element: Element) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            match chr {
                '"' => {
                    self.states.push(States::Str(StringState::Normal));
                    self.advance();
                    self.forward()?;
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                '0'..='9' => {
                    self.states.push(States::Number);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                't' | 'f' => {
                    self.states.push(States::Bool);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                'n' => {
                    self.states.push(States::Null);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                '[' => {
                    self.states.push(States::Array(0));
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::Value(Element::Index(0)));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward()?;
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                '{' => {
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward()?;
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx, self.display_path())))
                }
                ']' | '}' => {
                    // End of an array or object -> no value matched
                    Ok(None)
                }
                _ => Err(error::Generic),
            }
        } else {
            self.states.push(States::Value(element));
            Ok(Some(Output::Pending))
        }
    }

    fn process_str(&mut self, state: StringState) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            match chr {
                '"' => {
                    if state == StringState::Normal {
                        self.forward()?;
                        self.advance();
                        let prev_path = self.display_path();
                        self.path.pop().ok_or_else(|| error::Generic)?;
                        Ok(Some(Output::End(self.total_idx, prev_path)))
                    } else {
                        self.forward()?;
                        self.states.push(States::Str(StringState::Normal));
                        Ok(None)
                    }
                }
                '\\' => {
                    self.forward()?;
                    let new_state = match state {
                        StringState::Escaped => StringState::Normal,
                        StringState::Normal => StringState::Escaped,
                    };
                    self.states.push(States::Str(new_state));
                    Ok(None)
                }
                _ => {
                    self.forward()?;
                    self.states.push(States::Str(StringState::Normal));
                    Ok(None)
                }
            }
        } else {
            self.states.push(States::Str(state));
            Ok(Some(Output::Pending))
        }
    }

    fn process_number(&mut self) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            if chr.is_digit(10) || chr == '.' {
                self.forward()?;
                self.states.push(States::Number);
                Ok(None)
            } else {
                self.advance();
                let prev_path = self.display_path();
                self.path.pop().ok_or_else(|| error::Generic)?;
                Ok(Some(Output::End(self.total_idx, prev_path)))
            }
        } else {
            self.states.push(States::Number);
            Ok(Some(Output::Pending))
        }
    }

    fn process_bool(&mut self) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            if chr.is_alphabetic() {
                self.forward()?;
                self.states.push(States::Bool);
                Ok(None)
            } else {
                self.advance();
                let prev_path = self.display_path();
                self.path.pop().ok_or_else(|| error::Generic)?;
                Ok(Some(Output::End(self.total_idx, prev_path)))
            }
        } else {
            self.states.push(States::Bool);
            Ok(Some(Output::Pending))
        }
    }

    fn process_null(&mut self) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            if chr.is_alphabetic() {
                self.forward()?;
                self.states.push(States::Null);
                Ok(None)
            } else {
                self.advance();
                let prev_path = self.display_path();
                self.path.pop().ok_or_else(|| error::Generic)?;
                Ok(Some(Output::End(self.total_idx, prev_path)))
            }
        } else {
            self.states.push(States::Null);
            Ok(Some(Output::Pending))
        }
    }

    fn process_array(&mut self, idx: usize) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            match chr {
                ']' => {
                    self.forward()?;
                    self.advance();
                    let exported_path = self.display_path();
                    self.path.pop().ok_or(error::Generic)?;
                    Ok(Some(Output::End(self.total_idx, exported_path)))
                }
                ',' => {
                    self.forward()?;
                    self.states.push(States::Array(idx + 1));
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::Value(Element::Index(idx + 1)));
                    self.states.push(States::RemoveWhitespaces);
                    Ok(None)
                }
                _ => Err(error::Generic),
            }
        } else {
            self.states.push(States::Array(idx));
            Ok(Some(Output::Pending))
        }
    }

    fn process_object(&mut self) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            match chr {
                '}' => {
                    self.forward()?;
                    self.advance();
                    let exported_path = self.display_path();
                    self.path.pop().ok_or_else(|| error::Generic)?;
                    Ok(Some(Output::End(self.total_idx, exported_path)))
                }
                ',' => {
                    self.forward()?;
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    Ok(None)
                }
                _ => Err(error::Generic),
            }
        } else {
            self.states.push(States::Object);
            Ok(Some(Output::Pending))
        }
    }

    fn process_object_key(
        &mut self,
        state: ObjectKeyState,
    ) -> Result<Option<Output>, error::Generic> {
        match state {
            ObjectKeyState::Init => {
                if let Some(chr) = self.peek()? {
                    match chr {
                        '"' => {
                            self.advance(); // move cursor to the start
                            self.forward()?;
                            self.states.push(States::ObjectKey(ObjectKeyState::Parse(
                                StringState::Normal,
                            )));
                            Ok(None)
                        }
                        '}' => Ok(None),          // end has been reached to Object
                        _ => Err(error::Generic), // keys are strings in JSON
                    }
                } else {
                    self.states.push(States::ObjectKey(state));
                    Ok(Some(Output::Pending))
                }
            }
            ObjectKeyState::Parse(string_state) => {
                if let Some(chr) = self.peek()? {
                    self.forward()?;
                    match string_state {
                        StringState::Normal => match chr {
                            '\"' => {
                                let key = from_utf8(&self.pending[1..self.pending_idx - 1])
                                    .map_err(|_| error::Generic)?
                                    .to_string();
                                self.states.push(States::Value(Element::Key(key)));
                                self.states.push(States::RemoveWhitespaces);
                                self.states.push(States::Colon);
                                self.states.push(States::RemoveWhitespaces);
                                Ok(None)
                            }
                            '\\' => {
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

    fn process_colon(&mut self) -> Result<Option<Output>, error::Generic> {
        if let Some(chr) = self.peek()? {
            if chr != ':' {
                return Err(error::Generic);
            }
            self.forward()?;
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
    pub fn read(&mut self) -> Result<Output, error::Generic> {
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

    #[test]
    fn test_utf8_error() {
        let error_occured = |mut emitter: Emitter| loop {
            match emitter.read() {
                Ok(Output::Pending) | Ok(Output::Finished) => {
                    return false;
                }
                Ok(_) => (),
                Err(_) => {
                    return true;
                }
            }
        };

        let error_inputs = vec![
            vec![b'"', 0b1000_0000, b'"'],
            vec![b'"', 0b1111_1000, b'"'],
            vec![b'"', 0b1100_0000, b'1', b'"'],
            vec![b'"', 0b1110_0000, 0b1000_0000, b'2', b'"'],
            vec![b'"', 0b1111_0000, 0b1000_0000, 0b1000_0000, b'3', b'"'],
        ];
        for input in error_inputs {
            let mut emitter = Emitter::new();
            emitter.feed(&input[..]);
            assert!(error_occured(emitter));
        }
    }
}
