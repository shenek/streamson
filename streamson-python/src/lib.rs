use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::wrap_pyfunction;
use pyo3::types::{PyBytes, PyTuple};
use pyo3::exceptions;

use std::sync::{Arc, Mutex};
use streamson_lib::{error, handler, matcher, Collector};

create_exception!(streamson, StreamsonError, exceptions::ValueError);

impl From<error::General> for StreamsonError {
    fn from(gerror: error::General) -> Self {
        Self
    }
}

/// Low level Python wrapper for Simple matcher and Buffer handler
#[pyclass]
pub struct SimpleStreamson {
    collector: Collector,
    handler: Arc<Mutex<handler::Buffer>>,
}

#[pymethods]
impl SimpleStreamson {

    /// Create a new instance of SimpleStreamson
    ///
    /// # Arguments
    /// * `matches` - a list of valid simple matches (e.g. `{"users"}`, `[]{"name"}`, `[0]{}`)
    #[new]
    pub fn new(matches: Vec<String>) -> Self {
        let handler = Arc::new(Mutex::new(handler::Buffer::new()));
        let mut collector = Collector::new();
        for path_match in matches {
            collector = collector.add_matcher(
                Box::new(matcher::Simple::new(path_match)),
                &[handler.clone()],
            );
        }
        Self { collector, handler }
    }

    /// Feeds Streamson processor with data
    ///
    /// # Arguments
    /// * `data` - input data to be processed
    pub fn feed(&mut self, data: &[u8]) -> PyResult<()> {
        if let Err(err) = self.collector.process(data) {
            Err(StreamsonError::from(err).into())
        } else {
            Ok(())
        }
    }

    /// Reads data from Buffer handler
    ///
    /// # Returns
    /// * `None` - if no data present
    /// * `Some(<path>, <bytes>)` if there are some data
    fn pop(&mut self) -> Option<(String, Vec<u8>)>{

        match self.handler.lock().unwrap().pop() {
            Some((path, bytes)) => {
                Some((path, bytes.to_vec()))
            },
            None => None,
        }

    }
}
/// This module is a python module implemented in Rust.
#[pymodule]
fn streamson(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<SimpleStreamson>()?;

    Ok(())
}
