//! Handler which groups multiple handlers together
//!
//!
//! ```
//! use streamson_lib::{handler, matcher, strategy::{self, Strategy}};
//! use std::sync::{Arc, Mutex};
//!
//!
//! let group_handler = handler::Group::new()
//!     .add_handler(Arc::new(Mutex::new(handler::Unstringify::new())))
//!     .add_handler(Arc::new(Mutex::new(handler::PrintLn::new())));
//!
//! let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
//! let mut trigger = strategy::Trigger::new();
//! trigger.add_matcher(Box::new(matcher), Arc::new(Mutex::new(group_handler)));
//!
//! for input in vec![
//!     br#"{"users": [{"id": 1, "name": "first"}, {"#.to_vec(),
//!     br#""id": 2, "name": "second}]}"#.to_vec(),
//! ] {
//!     trigger.process(&input).unwrap();
//! }
//!
//! ```
//!

use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use crate::{error, path::Path, streamer::Token};

use super::Handler;

/// A structure which groups handlers and determines a way how handlers are triggered
#[derive(Default, Clone)]
pub struct Group {
    handlers: Vec<Arc<Mutex<dyn Handler>>>,
}

impl Group {
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a handler to handler group (builder pattern)
    ///
    /// # Arguments
    /// * `handler` - handler to add
    ///
    /// # Returns
    /// * Group handler
    pub fn add_handler(mut self, handler: Arc<Mutex<dyn Handler>>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Adds a handler to handler group (mut reference)
    ///
    /// # Arguments
    /// * `handler` - handler to add
    pub fn add_handler_mut(&mut self, handler: Arc<Mutex<dyn Handler>>) {
        self.handlers.push(handler);
    }

    /// Iterates through handlers
    pub fn subhandlers(&self) -> &[Arc<Mutex<dyn Handler>>] {
        &self.handlers
    }
}

impl Handler for Group {
    fn start(
        &mut self,
        path: &Path,
        matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let mut result = None;
        for handler in self.handlers.iter() {
            let mut guard = handler.lock().unwrap();
            if guard.is_converter() {
                let orig_result = result.take();
                result = guard.start(path, matcher_idx, token.clone())?;
                if let Some(orig_data) = orig_result {
                    let feed_output = guard.feed(&orig_data, matcher_idx)?;
                    if let Some(mut data) = result.take() {
                        if let Some(feed_data) = feed_output {
                            data.extend(feed_data);
                            result = Some(data);
                        }
                    } else {
                        result = feed_output;
                    }
                }
            } else {
                guard.start(path, matcher_idx, token.clone())?;
                if let Some(data) = result.as_ref() {
                    guard.feed(data, matcher_idx)?;
                }
            }
        }
        Ok(result)
    }

    fn feed(&mut self, data: &[u8], matcher_idx: usize) -> Result<Option<Vec<u8>>, error::Handler> {
        let mut result = Some(data.to_vec());
        for handler in self.handlers.iter() {
            let mut guard = handler.lock().unwrap();
            if let Some(data) = result.take() {
                if guard.is_converter() {
                    result = guard.feed(&data, matcher_idx)?;
                } else {
                    guard.feed(&data, matcher_idx)?;
                    result = Some(data)
                }
            } else {
                // data were consumed
                break;
            }
        }
        Ok(result)
    }

    fn end(
        &mut self,
        path: &Path,
        matcher_idx: usize,
        token: Token,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let mut result: Option<Vec<u8>> = None;
        for handler in self.handlers.iter() {
            let mut guard = handler.lock().unwrap();
            if guard.is_converter() {
                // Feed with data if there are some data remaining
                if let Some(data) = result.take() {
                    result = guard.feed(&data, matcher_idx)?;
                }

                if let Some(data) = guard.end(path, matcher_idx, token.clone())? {
                    if let Some(mut result_data) = result.take() {
                        result_data.extend(data);
                        result = Some(result_data);
                    } else {
                        result = Some(data);
                    }
                }
            } else {
                if let Some(data) = result.as_ref() {
                    guard.feed(data, matcher_idx)?;
                }
                guard.end(path, matcher_idx, token.clone())?;
            }
        }
        Ok(result)
    }

    fn is_converter(&self) -> bool {
        self.handlers
            .iter()
            .any(|e| e.lock().unwrap().is_converter())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Group;
    use crate::{
        handler::{Buffer, Replace, Shorten},
        matcher::Simple,
        strategy::{Convert, Filter, OutputConverter, Strategy, Trigger},
    };
    use std::sync::{Arc, Mutex};

    fn prepare_handlers() -> (
        Arc<Mutex<Buffer>>,
        Arc<Mutex<Buffer>>,
        Arc<Mutex<Buffer>>,
        Arc<Mutex<Replace>>,
        Arc<Mutex<Shorten>>,
    ) {
        (
            Arc::new(Mutex::new(Buffer::new())),
            Arc::new(Mutex::new(Buffer::new())),
            Arc::new(Mutex::new(Buffer::new())),
            Arc::new(Mutex::new(Replace::new(br#""ccccc""#.to_vec()))),
            Arc::new(Mutex::new(Shorten::new(3, r#"..""#.into()))),
        )
    }

    #[test]
    fn test_convert() {
        let mut convert = Convert::new();
        let (buffer1, buffer2, buffer3, replace, shorten) = prepare_handlers();
        let matcher = Simple::new(r#"[]{"desc"}"#).unwrap();
        let group = Group::new()
            .add_handler(buffer1.clone())
            .add_handler(replace.clone())
            .add_handler(buffer2.clone())
            .add_handler(shorten.clone())
            .add_handler(buffer3.clone());

        convert.add_matcher(Box::new(matcher), Arc::new(Mutex::new(group)));

        let output = OutputConverter::new()
            .convert(
                &convert
                    .process(br#"[{"desc": "aa"}, {"desc": "bbbbbb"}]"#)
                    .unwrap(),
            )
            .into_iter()
            .map(|e| e.1)
            .collect::<Vec<Vec<u8>>>();

        // output
        assert_eq!(
            String::from_utf8(output.into_iter().flatten().collect()).unwrap(),
            r#"[{"desc": "ccc.."}, {"desc": "ccc.."}]"#
        );

        // buffer1
        assert_eq!(
            String::from_utf8(buffer1.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""aa""#
        );
        assert_eq!(
            String::from_utf8(buffer1.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""bbbbbb""#
        );
        assert!(buffer1.lock().unwrap().pop().is_none());

        // buffer2
        assert_eq!(
            String::from_utf8(buffer2.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccccc""#
        );
        assert_eq!(
            String::from_utf8(buffer2.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccccc""#
        );
        assert!(buffer2.lock().unwrap().pop().is_none());

        // buffer3
        assert_eq!(
            String::from_utf8(buffer3.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccc..""#
        );
        assert_eq!(
            String::from_utf8(buffer3.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccc..""#
        );
        assert!(buffer3.lock().unwrap().pop().is_none());
    }

    #[test]
    fn test_trigger() {
        let mut trigger = Trigger::new();
        let (buffer1, buffer2, buffer3, replace, shorten) = prepare_handlers();
        let matcher = Simple::new(r#"[]{"desc"}"#).unwrap();
        let group = Group::new()
            .add_handler(buffer1.clone())
            .add_handler(replace.clone())
            .add_handler(buffer2.clone())
            .add_handler(shorten.clone())
            .add_handler(buffer3.clone());

        trigger.add_matcher(Box::new(matcher), Arc::new(Mutex::new(group)));

        trigger
            .process(br#"[{"desc": "aa"}, {"desc": "bbbbbb"}]"#)
            .unwrap();

        // buffer1
        assert_eq!(
            String::from_utf8(buffer1.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""aa""#
        );
        assert_eq!(
            String::from_utf8(buffer1.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""bbbbbb""#
        );
        assert!(buffer1.lock().unwrap().pop().is_none());

        // buffer2
        assert_eq!(
            String::from_utf8(buffer2.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccccc""#
        );
        assert_eq!(
            String::from_utf8(buffer2.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccccc""#
        );
        assert!(buffer2.lock().unwrap().pop().is_none());

        // buffer3
        assert_eq!(
            String::from_utf8(buffer3.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccc..""#
        );
        assert_eq!(
            String::from_utf8(buffer3.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccc..""#
        );
        assert!(buffer3.lock().unwrap().pop().is_none());
    }

    #[test]
    fn test_filter() {
        let mut filter = Filter::new();
        let (buffer1, buffer2, buffer3, replace, shorten) = prepare_handlers();
        let matcher = Simple::new(r#"[]{"desc"}"#).unwrap();
        let group = Group::new()
            .add_handler(buffer1.clone())
            .add_handler(replace.clone())
            .add_handler(buffer2.clone())
            .add_handler(shorten.clone())
            .add_handler(buffer3.clone());

        filter.add_matcher(Box::new(matcher), Some(Arc::new(Mutex::new(group))));

        let output: Vec<u8> = OutputConverter::new()
            .convert(
                &filter
                    .process(br#"[{"desc": "aa"}, {"desc": "bbbbbb"}]"#)
                    .unwrap(),
            )
            .into_iter()
            .map(|e| e.1)
            .flatten()
            .collect();

        // output
        assert_eq!(String::from_utf8(output).unwrap(), r#"[{}, {}]"#);

        // buffer1
        assert_eq!(
            String::from_utf8(buffer1.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""aa""#
        );
        assert_eq!(
            String::from_utf8(buffer1.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""bbbbbb""#
        );
        assert!(buffer1.lock().unwrap().pop().is_none());

        // buffer2
        assert_eq!(
            String::from_utf8(buffer2.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccccc""#
        );
        assert_eq!(
            String::from_utf8(buffer2.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccccc""#
        );
        assert!(buffer2.lock().unwrap().pop().is_none());

        // buffer3
        assert_eq!(
            String::from_utf8(buffer3.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccc..""#
        );
        assert_eq!(
            String::from_utf8(buffer3.lock().unwrap().pop().unwrap().1).unwrap(),
            r#""ccc..""#
        );
        assert!(buffer3.lock().unwrap().pop().is_none());
    }

    #[test]
    fn test_extract() {
        // TODO finish once extract can trigger handlers
    }
}
