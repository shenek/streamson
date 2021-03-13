//! The main logic processing all elements from JSON
//!
//! This strategy doesn't require any matchers
//! Handlers will be triggered on every element

use super::{Output, Strategy};
use crate::{
    error,
    handler::{Group, Handler},
    streamer::{Streamer, Token},
};
use std::sync::{Arc, Mutex};

/// Trigger handlers on every element
#[derive(Default)]
pub struct All {
    /// Should handlers be used for converting
    convert: bool,
    /// Input idx against total idx
    input_start: usize,
    /// Responsible for data extraction
    streamer: Streamer,
    /// List of handlers to be triggered
    handlers: Arc<Mutex<Group>>,
    /// Current json level
    level: usize,
}

impl Strategy for All {
    fn get_export_path(&self) -> bool {
        false
    }

    fn process(&mut self, input: &[u8]) -> Result<Vec<Output>, error::General> {
        self.streamer.feed(input);
        let mut inner_idx = 0;
        let mut result = vec![];
        loop {
            match self.streamer.read()? {
                Token::Start(idx, kind) => {
                    let path = self.streamer.current_path();

                    if self.level == 0 {
                        result.push(Output::Start(None));
                    }

                    let to = idx - self.input_start;
                    let mut guard = self.handlers.lock().unwrap();
                    if let Some(data) = guard.feed(&input[inner_idx..to], 0)? {
                        if self.convert {
                            result.push(Output::Data(data));
                        }
                    }
                    if let Some(data) = guard.start(path, 0, Token::Start(idx, kind))? {
                        if self.convert {
                            result.push(Output::Data(data));
                        }
                    }
                    self.level += 1;
                    inner_idx = to;
                }
                Token::End(idx, kind) => {
                    let path = self.streamer.current_path();

                    let to = idx - self.input_start;
                    let mut guard = self.handlers.lock().unwrap();
                    if let Some(data) = guard.feed(&input[inner_idx..to], 0)? {
                        if self.convert {
                            result.push(Output::Data(data));
                        }
                    }
                    if let Some(data) = guard.end(path, 0, Token::End(idx, kind))? {
                        if self.convert {
                            result.push(Output::Data(data));
                        }
                    }
                    inner_idx = to;
                    self.level -= 1;
                    if self.level == 0 {
                        result.push(Output::End);
                    }
                }
                Token::Pending => {
                    self.input_start += input.len();
                    let mut guard = self.handlers.lock().unwrap();
                    if let Some(data) = guard.feed(&input[inner_idx..], 0)? {
                        if self.convert {
                            result.push(Output::Data(data));
                        }
                    }
                    return Ok(if self.convert { result } else { vec![] });
                }
                Token::Separator(_) => {}
            }
        }
    }
}

impl All {
    /// Creates a new `All`
    ///
    /// It triggers handlers on all found elements
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets whether handlers should be actually used to converting data
    pub fn set_convert(&mut self, convert: bool) {
        self.convert = convert;
    }

    /// Adds a handler to `All`
    ///
    /// # Arguments
    /// * `handler` - handler to be triggers when path matches
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{strategy, matcher, handler};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut trigger = strategy::All::new();
    /// let handler = handler::Analyser::new();
    /// trigger.add_handler(
    ///     Arc::new(Mutex::new(handler))
    /// );
    /// ```
    pub fn add_handler(&mut self, handler: Arc<Mutex<dyn Handler>>) {
        self.handlers.lock().unwrap().add_handler_mut(handler);
    }
}

#[cfg(test)]
mod tests {
    use super::{All, Strategy};
    use crate::{
        handler::{Analyser, Replace},
        strategy::OutputConverter,
        test::{Single, Splitter, Window},
    };
    use rstest::*;
    use std::sync::{Arc, Mutex};

    fn get_input() -> Vec<u8> {
        br#"{"elements": [1, 2, 3, 4, [5, 6], {"another": null}]}"#.to_vec()
    }

    #[rstest(
        splitter,
        case::single(Box::new(Single::new())),
        case::window1(Box::new(Window::new(1))),
        case::window5(Box::new(Window::new(5))),
        case::window100(Box::new(Window::new(100)))
    )]
    fn no_convert(splitter: Box<dyn Splitter>) {
        for part in splitter.split(get_input()) {
            let mut all = All::new();
            let handler = Arc::new(Mutex::new(Analyser::new()));
            all.add_handler(handler.clone());
            for input in part {
                all.process(&input).unwrap();
            }

            let guard = handler.lock().unwrap();
            let results = guard.results();
            assert_eq!(results.len(), 5);
            assert_eq!(results[0], ("".into(), 1));
            assert_eq!(results[1], (r#"{"elements"}"#.into(), 1));
            assert_eq!(results[2], (r#"{"elements"}[]"#.into(), 6));
            assert_eq!(results[3], (r#"{"elements"}[][]"#.into(), 2));
            assert_eq!(results[4], (r#"{"elements"}[]{"another"}"#.into(), 1));
        }
    }

    #[rstest(
        splitter,
        case::single(Box::new(Single::new())),
        case::window1(Box::new(Window::new(1))),
        case::window5(Box::new(Window::new(5))),
        case::window100(Box::new(Window::new(100)))
    )]
    fn convert(splitter: Box<dyn Splitter>) {
        for part in splitter.split(get_input()) {
            let mut all = All::new();
            all.set_convert(true);
            let handler = Arc::new(Mutex::new(Replace::new(br#"."#.to_vec())));
            all.add_handler(handler);
            let mut result = vec![];
            let mut converter = OutputConverter::new();
            for input in part {
                let output = converter.convert(&all.process(&input).unwrap());
                for data in output {
                    result.extend(data.1);
                }
            }
            dbg!(String::from_utf8(result.clone()).unwrap());

            assert_eq!(result, br#"..........."#);
        }
    }
}
