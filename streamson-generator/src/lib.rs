#![crate_name = "streamson_generator"]
#![feature(generators, generator_trait)]

//! Library which integrates `streamson-lib` with rust `generators`
//!
use std::{
    ops::{Generator, GeneratorState},
    pin::Pin,
    sync::{Arc, Mutex},
};

use streamson_lib::{error::General as StreamsonError, handler, matcher, strategy};

/// Wraps streamson extraction around a generator
///
/// # Example
/// ```
/// #![feature(generators, generator_trait)]
///
/// use std::{
///     fs,
///     io::{self, Read},
///     str::FromStr,
///     pin::Pin,
///     ops::{Generator, GeneratorState},
/// };
/// use streamson_generator::StreamsonGenerator;
/// use streamson_lib::matcher::Simple;
///
/// fn process_json() -> io::Result<()> {
///     let mut file = fs::File::open("/tmp/large.json")?;
///     let mut input_generator = move || {
///         loop {
///             let mut buffer = vec![0; 2048];
///             if file.read(&mut buffer).unwrap() == 0 {
///                 break;
///             }
///             yield buffer;
///         }
///     };
///
///     let matcher = Box::new(Simple::from_str(r#"{"users"}[]{"name"}"#).unwrap());
///     let mut output_generator = StreamsonGenerator::new(input_generator, matcher);
///
///     for item in output_generator {
///         match item {
///             Ok((path, data)) => {
///                 // Do something with the data
///             },
///             Err(err) => {
///                 // Deal with error situation
///             }
///         }
///     }
///
///     Ok(())
/// }
///
/// ```
pub struct StreamsonGenerator<G>
where
    G: Generator<Yield = Vec<u8>, Return = ()> + Unpin,
{
    input_generator: G,
    trigger: Arc<Mutex<strategy::Trigger>>,
    buffer: Arc<Mutex<handler::Buffer>>,
    error_occured: bool,
    exitting: bool,
}

impl<G> StreamsonGenerator<G>
where
    G: Generator<Yield = Vec<u8>, Return = ()> + Unpin,
{
    pub fn new(input_generator: G, matcher: Box<dyn matcher::MatchMaker>) -> Self {
        let mut trigger = strategy::Trigger::new();
        let buffer = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));
        trigger.add_matcher(matcher, &[buffer.clone()]);
        Self {
            input_generator,
            trigger: Arc::new(Mutex::new(trigger)),
            buffer,
            error_occured: false,
            exitting: false,
        }
    }
}

impl<G> Generator for StreamsonGenerator<G>
where
    G: Generator<Yield = Vec<u8>, Return = ()> + Unpin,
{
    type Yield = Result<(String, Vec<u8>), StreamsonError>;
    type Return = ();

    fn resume(mut self: Pin<&mut Self>, _arg: ()) -> GeneratorState<Self::Yield, Self::Return> {
        if self.error_occured {
            // Don't continue on error
            return GeneratorState::Complete(());
        }
        loop {
            // Try to pop buffer first
            let data = self.buffer.lock().unwrap().pop();
            if let Some((path, data)) = data {
                return GeneratorState::Yielded(Ok((path.unwrap(), data)));
            }

            if self.exitting {
                // Entire json parsed
                return GeneratorState::Complete(());
            }

            // Feed the buffer
            let input = Pin::new(&mut self.input_generator).resume(());
            match input {
                GeneratorState::Yielded(bytes) => {
                    let process_res = self.trigger.lock().unwrap().process(&bytes);
                    match process_res {
                        Ok(()) => continue,
                        Err(err) => {
                            self.error_occured = true;
                            return GeneratorState::Yielded(Err(err));
                        }
                    }
                }
                GeneratorState::Complete(_) => {
                    self.exitting = true;
                }
            }
        }
    }
}

impl<G> Iterator for StreamsonGenerator<G>
where
    G: Generator<Yield = Vec<u8>, Return = ()> + Unpin,
{
    type Item = Result<(String, Vec<u8>), StreamsonError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error_occured {
            return None;
        }

        match Pin::new(self).resume(()) {
            GeneratorState::Yielded(res) => Some(res),
            GeneratorState::Complete(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StreamsonGenerator;

    use std::str::FromStr;
    use streamson_lib::matcher;

    #[test]
    fn test_empty() {
        let input_generator = move || {
            for line in &[b"{", b"}"] {
                yield line.to_vec();
            }
        };

        let matcher = Box::new(matcher::Simple::from_str(r#"{"users"}[]{"name"}"#).unwrap());
        let mut wrapped_generator = StreamsonGenerator::new(input_generator, matcher);

        assert!(wrapped_generator.next().is_none());
    }

    #[test]
    fn test_basic() {
        let input = &[
            b"{".to_vec(),
            br#""users": ["#.to_vec(),
            br#"{"name": "user1"},"#.to_vec(),
            br#"{"name": "user2"},"#.to_vec(),
            br#"{"name": "user3"}"#.to_vec(),
            b"]".to_vec(),
            b"}".to_vec(),
        ];
        let input_generator = move || {
            for line in input {
                yield line.clone();
            }
        };

        let matcher = Box::new(matcher::Simple::from_str(r#"{"users"}[]{"name"}"#).unwrap());
        let mut wrapped_generator = StreamsonGenerator::new(input_generator, matcher);

        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[0]{"name"}"#.to_string(),
                br#""user1""#.to_vec()
            )
        );
        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[1]{"name"}"#.to_string(),
                br#""user2""#.to_vec()
            )
        );
        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[2]{"name"}"#.to_string(),
                br#""user3""#.to_vec()
            )
        );
        assert!(wrapped_generator.next().is_none());
    }

    #[test]
    fn test_multiple_input() {
        let input = &[
            br#"{"users": [{"name": "user1"},{"name": "user2"},{"name": "user3"}]}"#.to_vec(),
            br#"{"users": [{"name": "user4"},{"name": "user5"}]}"#.to_vec(),
        ];
        let input_generator = move || {
            for line in input {
                yield line.clone();
            }
        };

        let matcher = Box::new(matcher::Simple::from_str(r#"{"users"}[]{"name"}"#).unwrap());
        let mut wrapped_generator = StreamsonGenerator::new(input_generator, matcher);

        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[0]{"name"}"#.to_string(),
                br#""user1""#.to_vec()
            )
        );
        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[1]{"name"}"#.to_string(),
                br#""user2""#.to_vec()
            )
        );
        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[2]{"name"}"#.to_string(),
                br#""user3""#.to_vec()
            )
        );
        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[0]{"name"}"#.to_string(),
                br#""user4""#.to_vec()
            )
        );
        assert_eq!(
            wrapped_generator.next().unwrap().unwrap(),
            (
                r#"{"users"}[1]{"name"}"#.to_string(),
                br#""user5""#.to_vec()
            )
        );
        assert!(wrapped_generator.next().is_none());
    }
}
