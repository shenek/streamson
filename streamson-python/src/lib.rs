use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::wrap_pyfunction;
use pyo3::types::{PyBytes, PyTuple};
use pyo3::exceptions;
use std::sync::{Arc, Mutex};
use streamson_lib::{error, handler, matcher, Collector};

/// Python error mapped to streamson error
create_exception!(streamson, StreamsonError, exceptions::ValueError);

impl From<erorr::General> for StreamsonError {
    fn from(gerror: error::General) -> Self {
        Self
    }
}

/// Streamson
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
    fn new(matches: Vec<String>) -> Self {
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

    fn feed(&mut self, data: Bytes) -> PyResult<()> {
        if let Err(err) = self.collector.process(data.as_bytes()) {
            Err(StreamsonError::from(err))
        } else {
            Ok(())
        }
    }
    /*
    fn read(&mut self) -> PyObject {

        match self.handler.lock().unwrap() {
            Some((path, bytes)) => {

            },
            None => PyObject::from_not_null
        }

    }
    */
}
/// This module is a python module implemented in Rust.
#[pymodule]
fn streamson(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<SimpleStreamson>()?;

    Ok(())
