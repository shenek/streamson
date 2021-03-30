use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::strategy::{self, Strategy};

use crate::{handlers, matchers};

pub fn prepare_convert_subcommand() -> App<'static> {
    App::new("convert")
        .about("Converts parts of JSON")
        .arg(matchers::matchers_arg())
        .arg(handlers::handlers_arg())
}

pub fn process_convert(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut convert = strategy::Convert::new();

    let hndlrs = handlers::parse_handlers(matches, "convert")?;
    for (group, matcher) in matchers::parse_matchers(matches)? {
        if let Some(handler) = hndlrs.get(&group) {
            convert.add_matcher(Box::new(matcher), Arc::new(Mutex::new(handler.clone())));
        }
    }

    let mut buffer = vec![];
    let mut converter = strategy::OutputConverter::new();
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output = converter
            .convert(&convert.process(&buffer[..size])?)
            .into_iter()
            .map(|e| e.1)
            .collect::<Vec<Vec<u8>>>();
        buffer.clear();
        for data in &output {
            stdout().write_all(data)?;
        }
    }

    // Input terminated try to hit strategy termination
    let output = converter
        .convert(&convert.terminate()?)
        .into_iter()
        .map(|e| e.1)
        .collect::<Vec<Vec<u8>>>();
    for data in &output {
        stdout().write_all(data)?;
    }

    Ok(())
}
