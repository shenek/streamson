//! Streams significant point of JSON from input data

use crate::{
    error,
    path::{Element, Path},
};
use std::{
    collections::{vec_deque::Drain, VecDeque},
    str::from_utf8,
};

/// Kind of output
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParsedKind {
    /// Object e.g. {}
    Obj,
    /// Array e.g. []
    Arr,
    /// String e.g. ""
    Str,
    /// Number e.g. 0
    Num,
    /// Null e.g. null
    Null,
    /// Bolean e.g. false
    Bool,
}

/// Structure which contains further info about matched data
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Path starts here
    Start(usize, ParsedKind),
    /// Path ends here
    End(usize, ParsedKind),
    /// Element separator idx (idx of `,` between array/object elements)
    Separator(usize),
    /// Needs more data
    Pending,
}

impl Token {
    pub fn is_end(&self) -> bool {
        matches!(self, Self::End(_, _))
    }
}

impl AsRef<str> for ParsedKind {
    fn as_ref(&self) -> &str {
        match self {
            ParsedKind::Obj => "object",
            ParsedKind::Arr => "array",
            ParsedKind::Str => "string",
            ParsedKind::Num => "number",
            ParsedKind::Bool => "boolean",
            ParsedKind::Null => "null",
        }
    }
}

/// Key parsing states
#[derive(Debug)]
enum ObjectKeyState {
    Init,
    Parse(StringState),
}

/// Parsing string states
#[derive(Debug, PartialEq)]
enum StringState {
    Normal,
    Escaped,
}

/// JSON processing states
#[derive(Debug)]
enum States {
    Value(Option<Element>),
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
/// Start( 0, ParsedKind::Obj) // Streamer.path == ""
/// Start( 1, ParsedKind::Arr) // Streamer.path == "{\"People\"}"
/// Start( 3, ParsedKind::Obj) // Streamer.path == "{\"People\"}[0]"
/// Start( 4, ParsedKind::Num) // Streamer.path == "{\"People\"}[0]{\"Height\"}"
/// End(   5, ParsedKind::Num)
/// Start( 6, ParsedKind::Num) // Streamer.path == "{\"People\"}[0]{\"Age\"}"
/// End(   7, ParsedKind::Num)
/// End(   8, ParsedKind::Obj)
/// Start( 9, ParsedKind::Obj) // Streamer.path == "{\"People\"}[1]"
/// Start(10, ParsedKind::Num) // Streamer.path == "{\"People\"}[1]{\"Height\"}"
/// End(  11, ParsedKind::Num)
/// Start(12, ParsedKind::Num) // Streamer.path == "{\"People\"}[1]{\"Age\"}"
/// End(  13, ParsedKind::Num)
/// End(  14, ParsedKind::Obj)
/// End(  15, ParsedKind::Arr)
/// End(  16, ParsedKind::Obj)
/// ```
#[derive(Debug)]
pub struct Streamer {
    /// Path stack
    path: Path,
    /// Paring elements stack
    states: Vec<States>,
    /// Pending buffer
    pending: VecDeque<u8>,
    /// Total index of pending buffer
    pending_idx: usize,
    /// Total index agains the first byte passed to input
    total_idx: usize,
    /// Indicator whether to pop path in the next read
    pop_path: bool,
}

impl Default for Streamer {
    fn default() -> Self {
        Self {
            path: Path::default(),
            states: vec![States::Value(None), States::RemoveWhitespaces],
            pending: VecDeque::new(),
            pending_idx: 0,
            total_idx: 0,
            pop_path: false,
        }
    }
}

impl Streamer {
    /// Creates a new instance of streamer
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns current path
    pub fn current_path(&mut self) -> &mut Path {
        &mut self.path
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

    /// Feed streamer with data
    pub fn feed(&mut self, input: &[u8]) {
        self.pending.extend(input);
    }

    /// Moves cursor forward while characters are whitespace
    fn process_remove_whitespace(&mut self) -> Option<Token> {
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_whitespace() {
                self.advance();
                return None;
            }
            self.forward();
        }
        self.states.push(States::RemoveWhitespaces);
        Some(Token::Pending)
    }

    /// Processes value which type will be determined later
    fn process_value(&mut self, element: Option<Element>) -> Result<Option<Token>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.states.push(States::Str(StringState::Normal));
                    self.advance();
                    self.forward();
                    if let Some(element) = element {
                        self.path.push(element);
                    }
                    Ok(Some(Token::Start(self.total_idx, ParsedKind::Str)))
                }
                b'0'..=b'9' => {
                    self.states.push(States::Number);
                    self.advance();
                    if let Some(element) = element {
                        self.path.push(element);
                    }
                    Ok(Some(Token::Start(self.total_idx, ParsedKind::Num)))
                }
                b't' | b'f' => {
                    self.states.push(States::Bool);
                    self.advance();
                    if let Some(element) = element {
                        self.path.push(element);
                    }
                    Ok(Some(Token::Start(self.total_idx, ParsedKind::Bool)))
                }
                b'n' => {
                    self.states.push(States::Null);
                    self.advance();
                    if let Some(element) = element {
                        self.path.push(element);
                    }
                    Ok(Some(Token::Start(self.total_idx, ParsedKind::Null)))
                }
                b'[' => {
                    self.states.push(States::Array(0));
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::Value(Some(Element::Index(0))));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward();
                    if let Some(element) = element {
                        self.path.push(element);
                    }
                    Ok(Some(Token::Start(self.total_idx, ParsedKind::Arr)))
                }
                b'{' => {
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward();
                    if let Some(element) = element {
                        self.path.push(element);
                    }
                    Ok(Some(Token::Start(self.total_idx, ParsedKind::Obj)))
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
            Ok(Some(Token::Pending))
        }
    }

    /// Processes string on the input
    fn process_str(&mut self, state: StringState) -> Option<Token> {
        if let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    if state == StringState::Normal {
                        self.forward();
                        self.advance();
                        Some(Token::End(self.total_idx, ParsedKind::Str))
                    } else {
                        self.forward();
                        self.states.push(States::Str(StringState::Normal));
                        None
                    }
                }
                b'\\' => {
                    self.forward();
                    let new_state = match state {
                        StringState::Escaped => StringState::Normal,
                        StringState::Normal => StringState::Escaped,
                    };
                    self.states.push(States::Str(new_state));
                    None
                }
                _ => {
                    self.forward();
                    self.states.push(States::Str(StringState::Normal));
                    None
                }
            }
        } else {
            self.states.push(States::Str(state));
            Some(Token::Pending)
        }
    }

    /// Processes the number
    fn process_number(&mut self) -> Option<Token> {
        if let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || byte == b'.' {
                self.forward();
                self.states.push(States::Number);
                None
            } else {
                self.advance();
                Some(Token::End(self.total_idx, ParsedKind::Num))
            }
        } else {
            self.states.push(States::Number);
            Some(Token::Pending)
        }
    }

    /// Processes bool
    fn process_bool(&mut self) -> Option<Token> {
        if let Some(byte) = self.peek() {
            if byte.is_ascii_alphabetic() {
                self.forward();
                self.states.push(States::Bool);
                None
            } else {
                self.advance();
                Some(Token::End(self.total_idx, ParsedKind::Bool))
            }
        } else {
            self.states.push(States::Bool);
            Some(Token::Pending)
        }
    }

    /// Processes null
    fn process_null(&mut self) -> Option<Token> {
        if let Some(byte) = self.peek() {
            if byte.is_ascii_alphabetic() {
                self.forward();
                self.states.push(States::Null);
                None
            } else {
                self.advance();
                Some(Token::End(self.total_idx, ParsedKind::Null))
            }
        } else {
            self.states.push(States::Null);
            Some(Token::Pending)
        }
    }

    /// Processes an array
    fn process_array(&mut self, idx: usize) -> Result<Option<Token>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b']' => {
                    self.forward();
                    self.advance();
                    Ok(Some(Token::End(self.total_idx, ParsedKind::Arr)))
                }
                b',' => {
                    self.forward();
                    self.states.push(States::Array(idx + 1));
                    self.states.push(States::RemoveWhitespaces);
                    self.states
                        .push(States::Value(Some(Element::Index(idx + 1))));
                    self.states.push(States::RemoveWhitespaces);
                    Ok(Some(Token::Separator(self.total_idx)))
                }
                byte => {
                    Err(error::IncorrectInput::new(byte, self.total_idx + self.pending_idx).into())
                }
            }
        } else {
            self.states.push(States::Array(idx));
            Ok(Some(Token::Pending))
        }
    }

    /// Processes and object
    fn process_object(&mut self) -> Result<Option<Token>, error::General> {
        if let Some(byte) = self.peek() {
            match byte {
                b'}' => {
                    self.forward();
                    self.advance();
                    Ok(Some(Token::End(self.total_idx, ParsedKind::Obj)))
                }
                b',' => {
                    self.forward();
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    Ok(Some(Token::Separator(self.total_idx)))
                }
                byte => {
                    Err(error::IncorrectInput::new(byte, self.total_idx + self.pending_idx).into())
                }
            }
        } else {
            self.states.push(States::Object);
            Ok(Some(Token::Pending))
        }
    }

    /// Processes object key
    fn process_object_key(
        &mut self,
        state: ObjectKeyState,
    ) -> Result<Option<Token>, error::General> {
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
                    Ok(Some(Token::Pending))
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
                                self.states.push(States::Value(Some(Element::Key(key))));
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
                    Ok(Some(Token::Pending))
                }
            }
        }
    }

    /// Processes a single colon
    fn process_colon(&mut self) -> Result<Option<Token>, error::General> {
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
            Ok(Some(Token::Pending))
        }
    }

    /// Reads data from streamer and emits [Token](enum.Token.html) struct
    ///
    /// # Errors
    ///
    /// If invalid JSON is passed and error may be emitted.
    /// Note that validity of input JSON is not checked.
    pub fn read(&mut self) -> Result<Token, error::General> {
        loop {
            while let Some(state) = self.states.pop() {
                if self.pop_path {
                    self.path.pop();
                    self.pop_path = false;
                }

                match state {
                    States::RemoveWhitespaces => {
                        if let Some(output) = self.process_remove_whitespace() {
                            return Ok(output);
                        }
                    }
                    States::Value(element) => {
                        if let Some(output) = self.process_value(element)? {
                            return Ok(output);
                        }
                        if self.states.is_empty() {
                            return Ok(Token::Pending);
                        }
                    }
                    States::Str(state) => {
                        if let Some(output) = self.process_str(state) {
                            self.pop_path = output.is_end();
                            return Ok(output);
                        }
                    }
                    States::Number => {
                        if let Some(output) = self.process_number() {
                            self.pop_path = output.is_end();
                            return Ok(output);
                        }
                    }
                    States::Bool => {
                        if let Some(output) = self.process_bool() {
                            self.pop_path = output.is_end();
                            return Ok(output);
                        }
                    }
                    States::Null => {
                        if let Some(output) = self.process_null() {
                            self.pop_path = output.is_end();
                            return Ok(output);
                        }
                    }
                    States::Array(idx) => {
                        if let Some(output) = self.process_array(idx)? {
                            self.pop_path = output.is_end();
                            return Ok(output);
                        }
                    }
                    States::Object => {
                        if let Some(output) = self.process_object()? {
                            self.pop_path = output.is_end();
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
            self.states.push(States::Value(None));
            self.states.push(States::RemoveWhitespaces);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ParsedKind, Streamer, Token};
    use crate::path::Path;
    use std::convert::TryFrom;

    fn make_path(path: &str) -> Path {
        Path::try_from(path).unwrap()
    }

    #[test]
    fn test_spaces() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"  "#);
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_string() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"  "test string \" \\\" [ ] {} , :\\""#);
        assert_eq!(streamer.read().unwrap(), Token::Start(2, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(36, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);

        let mut streamer = Streamer::new();
        streamer.feed(br#"" another one " "#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(15, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_number() {
        let mut streamer = Streamer::new();
        streamer.feed(br#" 3.24 "#);
        assert_eq!(streamer.read().unwrap(), Token::Start(1, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(5, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_bool() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"  true  "#);
        assert_eq!(streamer.read().unwrap(), Token::Start(2, ParsedKind::Bool));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(6, ParsedKind::Bool));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_null() {
        let mut streamer = Streamer::new();
        // TODO think of some better way to terminate the nulls/bools/numbers
        streamer.feed(br#"null"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);

        let mut streamer = Streamer::new();
        streamer.feed(br#"null  "#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(4, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_array() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"[ null, 33, "string" ]"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(2, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path("[0]"));
        assert_eq!(streamer.read().unwrap(), Token::End(6, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path("[0]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(6));
        assert_eq!(streamer.read().unwrap(), Token::Start(8, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        assert_eq!(streamer.read().unwrap(), Token::End(10, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(10));
        assert_eq!(streamer.read().unwrap(), Token::Start(12, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("[2]"));
        assert_eq!(streamer.read().unwrap(), Token::End(20, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("[2]"));
        assert_eq!(streamer.read().unwrap(), Token::End(22, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_array_pending() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"[ null, 3"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(2, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path("[0]"));
        assert_eq!(streamer.read().unwrap(), Token::End(6, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path("[0]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(6));
        assert_eq!(streamer.read().unwrap(), Token::Start(8, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        streamer.feed(br#"3,"#);
        assert_eq!(streamer.read().unwrap(), Token::End(10, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(10));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
        assert_eq!(streamer.current_path(), &make_path(""));
        streamer.feed(br#" "string" ]"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(12, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("[2]"));
        assert_eq!(streamer.read().unwrap(), Token::End(20, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("[2]"));
        assert_eq!(streamer.read().unwrap(), Token::End(22, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_empty_array() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"[]"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(2, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_array_in_array() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"[ [], 33, ["string" , 44], [  ]]"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(2, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path("[0]"));
        assert_eq!(streamer.read().unwrap(), Token::End(4, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path("[0]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(4));
        assert_eq!(streamer.read().unwrap(), Token::Start(6, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        assert_eq!(streamer.read().unwrap(), Token::End(8, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[1]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(8));
        assert_eq!(streamer.read().unwrap(), Token::Start(10, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path("[2]"));
        assert_eq!(streamer.read().unwrap(), Token::Start(11, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("[2][0]"));
        assert_eq!(streamer.read().unwrap(), Token::End(19, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("[2][0]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(20));
        assert_eq!(streamer.read().unwrap(), Token::Start(22, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[2][1]"));
        assert_eq!(streamer.read().unwrap(), Token::End(24, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("[2][1]"));
        assert_eq!(streamer.read().unwrap(), Token::End(25, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path("[2]"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(25));
        assert_eq!(streamer.read().unwrap(), Token::Start(27, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path("[3]"));
        assert_eq!(streamer.read().unwrap(), Token::End(31, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path("[3]"));
        assert_eq!(streamer.read().unwrap(), Token::End(32, ParsedKind::Arr));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_object() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"{"a":"a", "b" :  true , "c": null, " \" \\\" \\": 33}"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(5, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("{\"a\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(8, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path("{\"a\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(8));
        assert_eq!(streamer.read().unwrap(), Token::Start(17, ParsedKind::Bool));
        assert_eq!(streamer.current_path(), &make_path("{\"b\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(21, ParsedKind::Bool));
        assert_eq!(streamer.current_path(), &make_path("{\"b\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(22));
        assert_eq!(streamer.read().unwrap(), Token::Start(29, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path("{\"c\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(33, ParsedKind::Null));
        assert_eq!(streamer.current_path(), &make_path("{\"c\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(33));
        assert_eq!(streamer.read().unwrap(), Token::Start(50, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path(r#"{" \" \\\" \\"}"#));
        assert_eq!(streamer.read().unwrap(), Token::End(52, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path(r#"{" \" \\\" \\"}"#));
        assert_eq!(streamer.read().unwrap(), Token::End(53, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_empty_object() {
        let mut streamer = Streamer::new();
        streamer.feed(br#"{}"#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(2, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_object_in_object() {
        let mut streamer = Streamer::new();
        streamer.feed(br#" {"u": {}, "j": {"x": {  }, "y": 10}} "#);
        assert_eq!(streamer.read().unwrap(), Token::Start(1, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(7, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"u\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(9, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"u\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(9));
        assert_eq!(streamer.read().unwrap(), Token::Start(16, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Start(22, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"x\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(26, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"x\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(26));
        assert_eq!(streamer.read().unwrap(), Token::Start(33, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"y\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(35, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"y\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(36, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(37, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_complex_with_pending() {
        const COMPLEX_DATA: &[u8] = br#" [{"aha y": {}, "j": {"x": [{  }, [ {}, null ]], "y" : 10}}, null, 43, [ {"a": false} ] ]"#;

        // Split complex data into parts
        for i in 0..COMPLEX_DATA.len() {
            let start_data = &COMPLEX_DATA[0..i];
            let end_data = &COMPLEX_DATA[i..];
            let mut streamer = Streamer::new();

            // feed the first part
            streamer.feed(start_data);

            let mut terminate = false;
            // Gets next item and feed the rest of the data when pending
            let mut get_item = |path: Option<&str>| loop {
                match streamer.read() {
                    Ok(Token::Pending) => {
                        if terminate {
                            break Token::Pending;
                        } else {
                            terminate = true;
                            streamer.feed(end_data);
                        }
                        continue;
                    }
                    Ok(e) => {
                        if let Some(pth) = path {
                            assert_eq!(streamer.current_path(), &make_path(pth));
                        }
                        return e;
                    }
                    Err(_) => panic!("Error occured"),
                }
            };

            assert_eq!(get_item(Some("")), Token::Start(1, ParsedKind::Arr));
            assert_eq!(get_item(Some("[0]")), Token::Start(2, ParsedKind::Obj));
            assert_eq!(
                get_item(Some("[0]{\"aha y\"}")),
                Token::Start(12, ParsedKind::Obj)
            );
            assert_eq!(
                get_item(Some("[0]{\"aha y\"}")),
                Token::End(14, ParsedKind::Obj)
            );
            assert_eq!(get_item(None), Token::Separator(14));
            assert_eq!(
                get_item(Some("[0]{\"j\"}")),
                Token::Start(21, ParsedKind::Obj)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}")),
                Token::Start(27, ParsedKind::Arr)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[0]")),
                Token::Start(28, ParsedKind::Obj)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[0]")),
                Token::End(32, ParsedKind::Obj)
            );
            assert_eq!(get_item(None), Token::Separator(32));
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[1]")),
                Token::Start(34, ParsedKind::Arr)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[1][0]")),
                Token::Start(36, ParsedKind::Obj)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[1][0]")),
                Token::End(38, ParsedKind::Obj)
            );
            assert_eq!(get_item(None), Token::Separator(38));
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[1][1]")),
                Token::Start(40, ParsedKind::Null)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[1][1]")),
                Token::End(44, ParsedKind::Null)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}[1]")),
                Token::End(46, ParsedKind::Arr)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"x\"}")),
                Token::End(47, ParsedKind::Arr)
            );
            assert_eq!(get_item(None), Token::Separator(47));
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"y\"}")),
                Token::Start(55, ParsedKind::Num)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}{\"y\"}")),
                Token::End(57, ParsedKind::Num)
            );
            assert_eq!(
                get_item(Some("[0]{\"j\"}")),
                Token::End(58, ParsedKind::Obj)
            );
            assert_eq!(get_item(Some("[0]")), Token::End(59, ParsedKind::Obj));
            assert_eq!(get_item(None), Token::Separator(59));
            assert_eq!(get_item(Some("[1]")), Token::Start(61, ParsedKind::Null));
            assert_eq!(get_item(Some("[1]")), Token::End(65, ParsedKind::Null));
            assert_eq!(get_item(None), Token::Separator(65));
            assert_eq!(get_item(Some("[2]")), Token::Start(67, ParsedKind::Num));
            assert_eq!(get_item(Some("[2]")), Token::End(69, ParsedKind::Num));
            assert_eq!(get_item(None), Token::Separator(69));
            assert_eq!(get_item(Some("[3]")), Token::Start(71, ParsedKind::Arr));
            assert_eq!(get_item(Some("[3][0]")), Token::Start(73, ParsedKind::Obj));
            assert_eq!(
                get_item(Some("[3][0]{\"a\"}")),
                Token::Start(79, ParsedKind::Bool)
            );
            assert_eq!(
                get_item(Some("[3][0]{\"a\"}")),
                Token::End(84, ParsedKind::Bool)
            );
            assert_eq!(get_item(Some("[3][0]")), Token::End(85, ParsedKind::Obj));
            assert_eq!(get_item(Some("[3]")), Token::End(87, ParsedKind::Arr));
            assert_eq!(get_item(Some("")), Token::End(89, ParsedKind::Arr));
            assert_eq!(get_item(None), Token::Pending);
        }
    }

    #[test]
    fn test_utf8() {
        // try to cover all utf8 character lengths
        let utf8_data: Vec<u8> = r#"[{"š𐍈€": "€š𐍈"}, "𐍈€š"]"#.to_string().into_bytes();
        for i in 0..utf8_data.len() {
            let start_data = &utf8_data[0..i];
            let end_data = &utf8_data[i..];
            let mut streamer = Streamer::new();

            // feed the first part
            streamer.feed(start_data);

            let mut terminate = false;
            // Gets next item and feed the rest of the data when pending
            let mut get_item = |path: Option<&str>| loop {
                match streamer.read() {
                    Ok(Token::Pending) => {
                        if terminate {
                            break Token::Pending;
                        } else {
                            terminate = true;
                            streamer.feed(end_data);
                        }
                        continue;
                    }
                    Ok(e) => {
                        if let Some(pth) = path {
                            assert_eq!(streamer.current_path(), &make_path(pth));
                        }
                        return e;
                    }
                    Err(_) => panic!("Error occured"),
                }
            };

            assert_eq!(get_item(Some("")), Token::Start(0, ParsedKind::Arr));
            assert_eq!(get_item(Some("[0]")), Token::Start(1, ParsedKind::Obj));
            assert_eq!(
                get_item(Some("[0]{\"š𐍈€\"}")),
                Token::Start(15, ParsedKind::Str)
            );
            assert_eq!(
                get_item(Some("[0]{\"š𐍈€\"}")),
                Token::End(26, ParsedKind::Str)
            );
            assert_eq!(get_item(Some("[0]")), Token::End(27, ParsedKind::Obj));
            assert_eq!(get_item(None), Token::Separator(27));
            assert_eq!(get_item(Some("[1]")), Token::Start(29, ParsedKind::Str));
            assert_eq!(get_item(Some("[1]")), Token::End(40, ParsedKind::Str));
            assert_eq!(get_item(Some("")), Token::End(41, ParsedKind::Arr));
            assert_eq!(get_item(None), Token::Pending);
        }
    }

    #[test]
    fn test_multiple_input_flat() {
        let mut streamer = Streamer::new();
        streamer.feed(br#""first" "second""third""#);
        assert_eq!(streamer.read().unwrap(), Token::Start(0, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(7, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(8, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(16, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(16, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::End(23, ParsedKind::Str));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }

    #[test]
    fn test_newlines() {
        let mut streamer = Streamer::new();
        streamer.feed(
            br#" {
                "u": {},
                "j": {
                    "x": {  } ,
                    "y":10
                }
            } "#,
        );
        assert_eq!(streamer.read().unwrap(), Token::Start(1, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Start(24, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"u\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(26, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"u\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(26));
        assert_eq!(streamer.read().unwrap(), Token::Start(49, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Start(76, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"x\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(80, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"x\"}"));
        assert_eq!(streamer.read().unwrap(), Token::Separator(81));
        assert_eq!(streamer.read().unwrap(), Token::Start(107, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"y\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(109, ParsedKind::Num));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}{\"y\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(127, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path("{\"j\"}"));
        assert_eq!(streamer.read().unwrap(), Token::End(141, ParsedKind::Obj));
        assert_eq!(streamer.current_path(), &make_path(""));
        assert_eq!(streamer.read().unwrap(), Token::Pending);
    }
}
