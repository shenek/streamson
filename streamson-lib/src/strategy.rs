//! Collection of json processing strategies

pub mod all;
pub mod convert;
pub mod extract;
pub mod filter;
pub mod trigger;

pub use all::All;
pub use convert::Convert;
pub use extract::Extract;
pub use filter::Filter;
pub use trigger::Trigger;

use crate::{error, path::Path};
use std::mem;

#[derive(Debug, PartialEq)]
pub enum Output {
    Start(Option<Path>),
    Data(Vec<u8>),
    End,
}

#[derive(Default)]
pub struct OutputConverter {
    buffer: Vec<u8>,
    paths: Vec<Option<Path>>,
}

impl OutputConverter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn convert(&mut self, input: &[Output]) -> Vec<(Option<Path>, Vec<u8>)> {
        let mut res = vec![];
        for field in input {
            match field {
                Output::Start(path_opt) => {
                    self.paths.push(path_opt.clone());
                }
                Output::Data(data) => {
                    self.buffer.extend(data);
                }
                Output::End => {
                    let mut output = vec![];
                    mem::swap(&mut output, &mut self.buffer);
                    res.push((self.paths.pop().unwrap_or(None), output));
                }
            }
        }
        res
    }
}

pub trait Strategy {
    /// Processes input data
    ///
    /// # Arguments
    /// * `input` - input data
    ///
    /// # Returns
    /// * `Ok(_) processing passed
    /// * `Err(_)` - error occured during processing
    ///
    /// # Errors
    ///
    /// If parsing logic finds that JSON is not valid,
    /// it returns `error::General`.
    ///
    /// Note that streamson assumes that its input is a valid
    /// JSONs and if not, it still might be processed without an error.
    /// This is caused because streamson does not validate JSON.
    fn process(&mut self, input: &[u8]) -> Result<Vec<Output>, error::General>;

    /// Should be called when input data terminates
    ///
    /// # Returns
    /// * `Ok(_) processing passed
    /// * `Err(_)` - error occured during processing
    fn terminate(&mut self) -> Result<Vec<Output>, error::General>;

    /// Should be called when a json on input is entirely read
    ///
    /// # Returns
    /// * `Ok(_) processing passed
    /// * `Err(_)` - error occured during processing
    fn json_finished(&mut self) -> Result<Vec<Output>, error::General>;
}

#[cfg(test)]
mod test {
    use super::{Output, OutputConverter, Path};
    use std::convert::TryFrom;

    #[test]
    fn converter() {
        let mut converter = OutputConverter::new();
        let data = converter.convert(&[
            Output::Start(None),
            Output::Data(b"1234".to_vec()),
            Output::End,
        ]);
        assert_eq!(data, vec![(None, b"1234".to_vec())]);

        let data = converter.convert(&[
            Output::Start(Some(Path::try_from("").unwrap())),
            Output::Data(b"567".to_vec()),
        ]);
        assert_eq!(data, vec![]);

        let data = converter.convert(&[Output::Data(b"89".to_vec()), Output::End]);
        assert_eq!(
            data,
            vec![(Some(Path::try_from("").unwrap()), b"56789".to_vec())]
        );
    }
}
