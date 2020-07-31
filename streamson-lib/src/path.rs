//! Emits paths with indexes from input data

use crate::error;
use std::{
    collections::{vec_deque::Drain, VecDeque},
    convert::TryFrom,
    fmt,
    str::from_utf8,
};

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

/// Output of path processing
#[derive(Debug, PartialEq)]
pub enum Output {
    /// Path starts here
    Start(usize),
    /// Path ends here
    End(usize),
    /// Needs more data
    Pending,
    /// No data left in json
    Finished,
}

impl Output {
    pub fn is_end(&self) -> bool {
        match self {
            Self::End(_) => true,
            _ => false,
        }
    }
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
/// Start( 0) // Emitter.path == ""
/// Start( 1) // Emitter.path == "{\"People\"}"
/// Start( 3) // Emitter.path == "{\"People\"}[0]"
/// Start( 4) // Emitter.path == "{\"People\"}[0]{\"Height\"}"
/// End(   5)
/// Start( 6) // Emitter.path == "{\"People\"}[0]{\"Age\"}"
/// End(   7)
/// End(   8)
/// Start( 9) // Emitter.path == "{\"People\"}[1]"
/// Start(10) // Emitter.path == "{\"People\"}[1]{\"Height\"}"
/// End(  11)
/// Start(12) // Emitter.path == "{\"People\"}[1]{\"Age\"}"
/// End(  13)
/// End(  14)
/// End(  15)
/// End(  16)
/// Finished
/// ```
#[derive(Debug)]
pub struct Emitter {
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

impl Default for Emitter {
    fn default() -> Self {
        Self {
            path: Path::default(),
            states: vec![States::Value(Element::Root), States::RemoveWhitespaces],
            pending: VecDeque::new(),
            pending_idx: 0,
            total_idx: 0,
            pop_path: false,
        }
    }
}

impl Emitter {
    /// Creates a new path emitter
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
                    Ok(Some(Output::Start(self.total_idx)))
                }
                b'0'..=b'9' => {
                    self.states.push(States::Number);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx)))
                }
                b't' | b'f' => {
                    self.states.push(States::Bool);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx)))
                }
                b'n' => {
                    self.states.push(States::Null);
                    self.advance();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx)))
                }
                b'[' => {
                    self.states.push(States::Array(0));
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::Value(Element::Index(0)));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx)))
                }
                b'{' => {
                    self.states.push(States::Object);
                    self.states.push(States::RemoveWhitespaces);
                    self.states.push(States::ObjectKey(ObjectKeyState::Init));
                    self.states.push(States::RemoveWhitespaces);
                    self.advance();
                    self.forward();
                    self.path.push(element);
                    Ok(Some(Output::Start(self.total_idx)))
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
                        Ok(Some(Output::End(self.total_idx)))
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
                Ok(Some(Output::End(self.total_idx)))
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
                Ok(Some(Output::End(self.total_idx)))
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
                Ok(Some(Output::End(self.total_idx)))
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
                    Ok(Some(Output::End(self.total_idx)))
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
                    Ok(Some(Output::End(self.total_idx)))
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
            if self.pop_path {
                self.path.pop();
                self.pop_path = false;
            }

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
                        self.pop_path = output.is_end();
                        return Ok(output);
                    }
                }
                States::Number => {
                    if let Some(output) = self.process_number()? {
                        self.pop_path = output.is_end();
                        return Ok(output);
                    }
                }
                States::Bool => {
                    if let Some(output) = self.process_bool()? {
                        self.pop_path = output.is_end();
                        return Ok(output);
                    }
                }
                States::Null => {
                    if let Some(output) = self.process_null()? {
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
        Ok(Output::Finished)
    }
}

#[cfg(test)]
mod tests {
    use super::{Element, Emitter, Output, Path};
    use std::convert::TryFrom;

    fn make_path(path: &str) -> Path {
        Path::try_from(path).unwrap()
    }

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
        assert_eq!(emitter.read().unwrap(), Output::Start(2));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(36));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);

        let mut emitter = Emitter::new();
        emitter.feed(br#"" another one " "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(15));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_number() {
        let mut emitter = Emitter::new();
        emitter.feed(br#" 3.24 "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(1));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(5));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_bool() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"  true  "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(2));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(6));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_null() {
        let mut emitter = Emitter::new();
        // TODO think of some better way to terminate the nulls/bools/numbers
        emitter.feed(br#"null"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Pending);

        let mut emitter = Emitter::new();
        emitter.feed(br#"null  "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(4));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_array() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[ null, 33, "string" ]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Start(2));
        assert_eq!(emitter.current_path(), &make_path("[0]"));
        assert_eq!(emitter.read().unwrap(), Output::End(6));
        assert_eq!(emitter.current_path(), &make_path("[0]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(8));
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        assert_eq!(emitter.read().unwrap(), Output::End(10));
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(12));
        assert_eq!(emitter.current_path(), &make_path("[2]"));
        assert_eq!(emitter.read().unwrap(), Output::End(20));
        assert_eq!(emitter.current_path(), &make_path("[2]"));
        assert_eq!(emitter.read().unwrap(), Output::End(22));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_array_pending() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[ null, 3"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Start(2));
        assert_eq!(emitter.current_path(), &make_path("[0]"));
        assert_eq!(emitter.read().unwrap(), Output::End(6));
        assert_eq!(emitter.current_path(), &make_path("[0]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(8));
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        assert_eq!(emitter.read().unwrap(), Output::Pending);
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        emitter.feed(br#"3,"#);
        assert_eq!(emitter.read().unwrap(), Output::End(10));
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        assert_eq!(emitter.read().unwrap(), Output::Pending);
        assert_eq!(emitter.current_path(), &make_path(""));
        emitter.feed(br#" "string" ]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(12));
        assert_eq!(emitter.current_path(), &make_path("[2]"));
        assert_eq!(emitter.read().unwrap(), Output::End(20));
        assert_eq!(emitter.current_path(), &make_path("[2]"));
        assert_eq!(emitter.read().unwrap(), Output::End(22));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_empty_array() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(2));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_array_in_array() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"[ [], 33, ["string" , 44], [  ]]"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Start(2));
        assert_eq!(emitter.current_path(), &make_path("[0]"));
        assert_eq!(emitter.read().unwrap(), Output::End(4));
        assert_eq!(emitter.current_path(), &make_path("[0]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(6));
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        assert_eq!(emitter.read().unwrap(), Output::End(8));
        assert_eq!(emitter.current_path(), &make_path("[1]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(10));
        assert_eq!(emitter.current_path(), &make_path("[2]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(11));
        assert_eq!(emitter.current_path(), &make_path("[2][0]"));
        assert_eq!(emitter.read().unwrap(), Output::End(19));
        assert_eq!(emitter.current_path(), &make_path("[2][0]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(22));
        assert_eq!(emitter.current_path(), &make_path("[2][1]"));
        assert_eq!(emitter.read().unwrap(), Output::End(24));
        assert_eq!(emitter.current_path(), &make_path("[2][1]"));
        assert_eq!(emitter.read().unwrap(), Output::End(25));
        assert_eq!(emitter.current_path(), &make_path("[2]"));
        assert_eq!(emitter.read().unwrap(), Output::Start(27));
        assert_eq!(emitter.current_path(), &make_path("[3]"));
        assert_eq!(emitter.read().unwrap(), Output::End(31));
        assert_eq!(emitter.current_path(), &make_path("[3]"));
        assert_eq!(emitter.read().unwrap(), Output::End(32));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_object() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"{"a":"a", "b" :  true , "c": null, " \" \\\" \\": 33}"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Start(5));
        assert_eq!(emitter.current_path(), &make_path("{\"a\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(8));
        assert_eq!(emitter.current_path(), &make_path("{\"a\"}"));
        assert_eq!(emitter.read().unwrap(), Output::Start(17));
        assert_eq!(emitter.current_path(), &make_path("{\"b\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(21));
        assert_eq!(emitter.current_path(), &make_path("{\"b\"}"));
        assert_eq!(emitter.read().unwrap(), Output::Start(29));
        assert_eq!(emitter.current_path(), &make_path("{\"c\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(33));
        assert_eq!(emitter.current_path(), &make_path("{\"c\"}"));
        assert_eq!(emitter.read().unwrap(), Output::Start(50));
        assert_eq!(emitter.current_path(), &make_path(r#"{" \" \\\" \\"}"#));
        assert_eq!(emitter.read().unwrap(), Output::End(52));
        assert_eq!(emitter.current_path(), &make_path(r#"{" \" \\\" \\"}"#));
        assert_eq!(emitter.read().unwrap(), Output::End(53));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_empty_object() {
        let mut emitter = Emitter::new();
        emitter.feed(br#"{}"#);
        assert_eq!(emitter.read().unwrap(), Output::Start(0));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::End(2));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Finished);
    }

    #[test]
    fn test_object_in_object() {
        let mut emitter = Emitter::new();
        emitter.feed(br#" {"u": {}, "j": {"x": {  }, "y": 10}} "#);
        assert_eq!(emitter.read().unwrap(), Output::Start(1));
        assert_eq!(emitter.current_path(), &make_path(""));
        assert_eq!(emitter.read().unwrap(), Output::Start(7));
        assert_eq!(emitter.current_path(), &make_path("{\"u\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(9));
        assert_eq!(emitter.current_path(), &make_path("{\"u\"}"));
        assert_eq!(emitter.read().unwrap(), Output::Start(16));
        assert_eq!(emitter.current_path(), &make_path("{\"j\"}"));
        assert_eq!(emitter.read().unwrap(), Output::Start(22));
        assert_eq!(emitter.current_path(), &make_path("{\"j\"}{\"x\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(26));
        assert_eq!(emitter.current_path(), &make_path("{\"j\"}{\"x\"}"));
        assert_eq!(emitter.read().unwrap(), Output::Start(33));
        assert_eq!(emitter.current_path(), &make_path("{\"j\"}{\"y\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(35));
        assert_eq!(emitter.current_path(), &make_path("{\"j\"}{\"y\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(36));
        assert_eq!(emitter.current_path(), &make_path("{\"j\"}"));
        assert_eq!(emitter.read().unwrap(), Output::End(37));
        assert_eq!(emitter.current_path(), &make_path(""));
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
            let mut get_item = |path: Option<&str>| loop {
                match emitter.read() {
                    Ok(Output::Pending) => {
                        emitter.feed(end_data);
                        continue;
                    }
                    Ok(e) => {
                        if let Some(pth) = path {
                            assert_eq!(emitter.current_path(), &make_path(pth));
                        }
                        return e;
                    }
                    Err(_) => panic!("Error occured"),
                }
            };

            assert_eq!(get_item(Some("")), Output::Start(1));
            assert_eq!(get_item(Some("[0]")), Output::Start(2));
            assert_eq!(get_item(Some("[0]{\"aha y\"}")), Output::Start(12));
            assert_eq!(get_item(Some("[0]{\"aha y\"}")), Output::End(14));
            assert_eq!(get_item(Some("[0]{\"j\"}")), Output::Start(21));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}")), Output::Start(27));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[0]")), Output::Start(28));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[0]")), Output::End(32));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[1]")), Output::Start(34));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[1][0]")), Output::Start(36));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[1][0]")), Output::End(38));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[1][1]")), Output::Start(40));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[1][1]")), Output::End(44));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}[1]")), Output::End(46));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"x\"}")), Output::End(47));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"y\"}")), Output::Start(55));
            assert_eq!(get_item(Some("[0]{\"j\"}{\"y\"}")), Output::End(57));
            assert_eq!(get_item(Some("[0]{\"j\"}")), Output::End(58));
            assert_eq!(get_item(Some("[0]")), Output::End(59));
            assert_eq!(get_item(Some("[1]")), Output::Start(61));
            assert_eq!(get_item(Some("[1]")), Output::End(65));
            assert_eq!(get_item(Some("[2]")), Output::Start(67));
            assert_eq!(get_item(Some("[2]")), Output::End(69));
            assert_eq!(get_item(Some("[3]")), Output::Start(71));
            assert_eq!(get_item(Some("[3][0]")), Output::Start(73));
            assert_eq!(get_item(Some("[3][0]{\"a\"}")), Output::Start(79));
            assert_eq!(get_item(Some("[3][0]{\"a\"}")), Output::End(84));
            assert_eq!(get_item(Some("[3][0]")), Output::End(85));
            assert_eq!(get_item(Some("[3]")), Output::End(87));
            assert_eq!(get_item(Some("")), Output::End(89));
            assert_eq!(get_item(None), Output::Finished);
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
            let mut get_item = |path: Option<&str>| loop {
                match emitter.read() {
                    Ok(Output::Pending) => {
                        emitter.feed(end_data);
                        continue;
                    }
                    Ok(e) => {
                        if let Some(pth) = path {
                            assert_eq!(emitter.current_path(), &make_path(pth));
                        }
                        return e;
                    }
                    Err(_) => panic!("Error occured"),
                }
            };

            assert_eq!(get_item(Some("")), Output::Start(0));
            assert_eq!(get_item(Some("[0]")), Output::Start(1));
            assert_eq!(get_item(Some("[0]{\"š𐍈€\"}")), Output::Start(15));
            assert_eq!(get_item(Some("[0]{\"š𐍈€\"}")), Output::End(26));
            assert_eq!(get_item(Some("[0]")), Output::End(27));
            assert_eq!(get_item(Some("[1]")), Output::Start(29));
            assert_eq!(get_item(Some("[1]")), Output::End(40));
            assert_eq!(get_item(Some("")), Output::End(41));
            assert_eq!(get_item(None), Output::Finished);
        }
    }

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
