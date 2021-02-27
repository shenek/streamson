//! The main logic of JSON converting
//!
//! It substitutes a part of output with other data.
//!
//! Nested matches are not considered. Data are converted only by the
//! first match.

use crate::{
    error,
    handler::Handler,
    matcher::MatchMaker,
    path::Path,
    streamer::{Streamer, Token},
};
use std::sync::{Arc, Mutex};

/// Item in matcher list
type MatcherItem = (Box<dyn MatchMaker>, Arc<Mutex<dyn Handler>>);

/// Processes data from input and triggers handler
pub struct Convert {
    /// Input idx against total idx
    input_start: usize,
    /// Currently matched path and matcher index
    matched: Option<(Path, usize)>,
    /// Path matchers and a handler
    matchers: Vec<MatcherItem>,
    /// Responsible for data extraction
    streamer: Streamer,
}

impl Default for Convert {
    fn default() -> Self {
        Self {
            input_start: 0,
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
    /// * `handlers` - funtions which should be run to convert the data
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
    ///     Arc::new(Mutex::new(handler::Replace::new(vec![b'"', b'*', b'*', b'*', b'"']))),
    /// );
    /// ```
    pub fn add_matcher(&mut self, matcher: Box<dyn MatchMaker>, handler: Arc<Mutex<dyn Handler>>) {
        self.matchers.push((matcher, handler));
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
    /// use streamson_lib::{strategy, handler, matcher, path::Path};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut convert = strategy::Convert::new();
    /// let matcher = matcher::Simple::new(r#"{"password"}"#).unwrap();
    /// convert.add_matcher(
    ///     Box::new(matcher),
    ///     Arc::new(Mutex::new(handler::Replace::new(vec![b'"', b'*', b'*', b'*', b'"']))),
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
                Token::Start(idx, kind) => {
                    if self.matched.is_none() {
                        // try to check whether it matches
                        for (matcher_idx, (matcher, _)) in self.matchers.iter().enumerate() {
                            if matcher.match_path(self.streamer.current_path(), kind) {
                                // start collecting
                                self.matched =
                                    Some((self.streamer.current_path().clone(), matcher_idx));

                                // Flush remaining data to output
                                let to = idx - self.input_start;
                                result.push(input[inner_idx..to].to_vec());
                                inner_idx = to;

                                // Notify handler that match has started
                                let mut handler = self.matchers[matcher_idx].1.lock().unwrap();
                                if let Some(data) = handler.start(
                                    self.streamer.current_path(),
                                    matcher_idx,
                                    Token::Start(idx, kind),
                                )? {
                                    result.push(data);
                                }
                                break;
                            }
                        }
                    }
                }
                Token::End(idx, kind) => {
                    let mut clear = false;
                    if let Some((matched_path, matcher_idx)) = self.matched.take() {
                        if self.streamer.current_path() == &matched_path {
                            clear = true;

                            // move the buffer
                            let to = idx - self.input_start;
                            let data = &input[inner_idx..to];
                            inner_idx = to;

                            let mut handler = self.matchers[matcher_idx].1.lock().unwrap();

                            // consume the data
                            if let Some(to_output) = handler.feed(data, matcher_idx)? {
                                result.push(to_output);
                            }

                            // Notify handlers that match has ended
                            if let Some(data) = handler.end(
                                self.streamer.current_path(),
                                matcher_idx,
                                Token::Start(idx, kind),
                            )? {
                                result.push(data);
                            }
                        }
                        if !clear {
                            self.matched = Some((matched_path, matcher_idx));
                        }
                    }
                }
                Token::Pending => {
                    self.input_start += input.len();
                    if let Some((_, matcher_idx)) = self.matched {
                        let mut handler = self.matchers[matcher_idx].1.lock().unwrap();
                        if let Some(to_output) = handler.feed(&input[inner_idx..], matcher_idx)? {
                            result.push(to_output);
                        }
                    } else {
                        result.push(input[inner_idx..].to_vec())
                    }
                    return Ok(result);
                }
                Token::Separator(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Convert;
    use crate::{
        handler::{Group, Replace, Shorten},
        matcher::Simple,
    };
    use std::sync::{Arc, Mutex};

    fn make_replace_handler() -> Arc<Mutex<Replace>> {
        return Arc::new(Mutex::new(Replace::new(vec![b'"', b'*', b'*', b'*', b'"'])));
    }

    #[test]
    fn basic() {
        let mut convert = Convert::new();
        let matcher = Simple::new(r#"[]{"password"}"#).unwrap();
        convert.add_matcher(Box::new(matcher), make_replace_handler());

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
        convert.add_matcher(Box::new(matcher), make_replace_handler());

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

    #[test]
    fn chaining_handlers() {
        let mut convert = Convert::new();
        let matcher = Simple::new(r#"[]{"password"}"#).unwrap();
        let replace = Arc::new(Mutex::new(Replace::new(br#""*****************""#.to_vec())));
        let shorten = Arc::new(Mutex::new(Shorten::new(4, "...\"".into())));
        convert.add_matcher(
            Box::new(matcher),
            Arc::new(Mutex::new(
                Group::new().add_handler(replace).add_handler(shorten),
            )),
        );

        let output = convert
            .process(br#"[{"id": 1, "password": "secret1"}, {"id": 2, "password": "secret2"}]"#)
            .unwrap();

        assert_eq!(
            String::from_utf8(output.into_iter().flatten().collect()).unwrap(),
            r#"[{"id": 1, "password": "****..."}, {"id": 2, "password": "****..."}]"#
        );
    }
}
