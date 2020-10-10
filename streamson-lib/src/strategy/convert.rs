//! The main logic of JSON converting
//!
//! It substitutes a part of output with other data.
//!
//! Nested matches are not considered. Data are converted only by the
//! first match.

use crate::{
    error,
    matcher::MatchMaker,
    path::Path,
    streamer::{Output, Streamer},
};
use std::{
    mem,
    sync::{Arc, Mutex},
};

/// Convert function type
type ConvertFunction = Arc<Mutex<dyn Fn(&Path, &[u8]) -> Vec<u8>>>;

/// Item in matcher list
type MatcherItem = (Box<dyn MatchMaker>, ConvertFunction);

/// Processes data from input and triggers handlers
pub struct Convert {
    /// Input idx against total idx
    input_start: usize,
    /// Buffer index against total idx
    buffer_start: usize,
    /// Buffer which is used to store collected data
    buffer: Vec<u8>,
    /// Currently matched path and matcher index
    matched: Option<(Path, usize)>,
    /// Path matchers and handlers
    matchers: Vec<MatcherItem>,
    /// Responsible for data extraction
    streamer: Streamer,
}

impl Default for Convert {
    fn default() -> Self {
        Self {
            input_start: 0,
            buffer_start: 0,
            buffer: vec![],
            matched: None,
            matchers: vec![],
            streamer: Streamer::new(),
        }
    }
}

impl Convert {
    /// Creates a new `Convert`
    ///
    /// It should replace a parts of the JSON using custom bytes
    /// data are read.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a mathcher and a handler to `Convert`
    ///
    /// # Arguments
    /// * `matcher` - matcher which matches the path
    /// * `convert_function` - funtions which performs the conversion
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{strategy, matcher, handler, path::Path};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut convert = strategy::Convert::new();
    ///
    /// let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();
    /// convert.add_matcher(
    ///     Box::new(matcher),
    ///     Arc::new(Mutex::new(|_: &Path, _: &[u8]| vec![b'"', b'*', b'*', b'*', b'"'])),
    /// );
    /// ```
    pub fn add_matcher(&mut self, matcher: Box<dyn MatchMaker>, convert_function: ConvertFunction) {
        self.matchers.push((matcher, convert_function));
    }

    /// Processes input data
    ///
    /// # Arguments
    /// * `input` - input data
    ///
    /// # Returns
    /// * `Ok(_) processing passed, more data might be needed
    /// * `Err(_)` when input is not correct json
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{strategy, matcher, path::Path};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut convert = strategy::Convert::new();
    /// let matcher = matcher::Simple::new(r#"{"password"}"#).unwrap();
    /// convert.add_matcher(
    ///     Box::new(matcher),
    ///     Arc::new(Mutex::new(|_: &Path, _: &[u8]| vec![b'"', b'*', b'*', b'*', b'"'])),
    /// );
    ///
    /// let data = convert.process(br#"{"password": "secret"}"#).unwrap();
    /// for part in data {
    ///     // Do something with converted data
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// If parsing logic finds that JSON is not valid,
    /// it returns `error::General`.
    pub fn process(&mut self, input: &[u8]) -> Result<Vec<Vec<u8>>, error::General> {
        self.streamer.feed(input);
        let mut inner_idx = 0;

        let mut result: Vec<Vec<u8>> = vec![];
        loop {
            match self.streamer.read()? {
                Output::Start(idx) => {
                    if self.matched.is_none() {
                        // try to check whether it matches
                        let path = self.streamer.current_path();
                        for (matcher_idx, (matcher, _)) in self.matchers.iter().enumerate() {
                            if matcher.match_path(path) {
                                // start collecting
                                self.buffer_start = idx;
                                self.matched = Some((path.clone(), matcher_idx));

                                // Flush data to output
                                let to = idx - self.input_start;
                                result.push(input[inner_idx..to].to_vec());
                                inner_idx = to;

                                break;
                            }
                        }
                    }
                }
                Output::End(idx) => {
                    let current_path = self.streamer.current_path();
                    let mut clear = false;
                    if let Some((matched_path, matcher_idx)) = self.matched.as_ref() {
                        if current_path == matched_path {
                            clear = true;

                            // move the buffer
                            let to = idx - self.input_start;
                            self.buffer.extend(&input[inner_idx..to]);
                            inner_idx = to;

                            // new buffer
                            let mut buffer = vec![];
                            mem::swap(&mut buffer, &mut self.buffer);

                            result.push(self.matchers[*matcher_idx].1.lock().unwrap()(
                                current_path,
                                &buffer,
                            ));
                        }
                    }
                    if clear {
                        self.matched = None;
                    }
                }
                Output::Pending => {
                    self.input_start += input.len();
                    if self.matched.is_some() {
                        self.buffer.extend(&input[inner_idx..]);
                    } else {
                        result.push(input[inner_idx..].to_vec())
                    }
                    return Ok(result);
                }
                Output::Separator(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Convert, ConvertFunction, Path};
    use crate::matcher::Simple;
    use std::sync::{Arc, Mutex};

    fn make_password_convert() -> ConvertFunction {
        return Arc::new(Mutex::new(|_: &Path, _: &[u8]| {
            vec![b'"', b'*', b'*', b'*', b'"']
        }));
    }

    #[test]
    fn basic() {
        let mut convert = Convert::new();
        let matcher = Simple::new(r#"[]{"password"}"#).unwrap();
        convert.add_matcher(Box::new(matcher), make_password_convert());

        let mut output = convert
            .process(br#"[{"id": 1, "password": "secret1"}, {"id": 2, "password": "secret2"}]"#)
            .unwrap();

        assert_eq!(output.len(), 5);
        assert_eq!(
            String::from_utf8(output.remove(0)).unwrap(),
            r#"[{"id": 1, "password": "#
        );
        assert_eq!(String::from_utf8(output.remove(0)).unwrap(), r#""***""#);
        assert_eq!(
            String::from_utf8(output.remove(0)).unwrap(),
            r#"}, {"id": 2, "password": "#
        );
        assert_eq!(String::from_utf8(output.remove(0)).unwrap(), r#""***""#);
        assert_eq!(String::from_utf8(output.remove(0)).unwrap(), "}]");
    }

    #[test]
    fn basic_pending() {
        let mut convert = Convert::new();
        let matcher = Simple::new(r#"[]{"password"}"#).unwrap();
        convert.add_matcher(Box::new(matcher), make_password_convert());

        let mut result = vec![];
        let output = convert.process(br#"[{"id": 1, "password": "s"#).unwrap();
        result.extend(output);

        let output = convert
            .process(br#"ecret1"}, {"id": 2, "password": "secret2"}]"#)
            .unwrap();
        result.extend(output);
        assert_eq!(
            String::from_utf8(result.into_iter().flatten().collect()).unwrap(),
            r#"[{"id": 1, "password": "***"}, {"id": 2, "password": "***"}]"#
        );
    }
}
