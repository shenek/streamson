use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::strategy::{self, Strategy};

use crate::{handlers, matchers};

pub fn prepare_filter_subcommand() -> App<'static> {
    App::new("filter")
        .about("Removes matched parts of JSON")
        .arg(matchers::matchers_arg())
        .arg(handlers::handlers_arg())
}

pub fn process_filter(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut filter = strategy::Filter::new();

    let hndlrs = handlers::parse_handlers(matches)?;

    for (group, matcher) in matchers::parse_matchers(matches)? {
        if let Some(handler) = hndlrs.get(&group) {
            filter.add_matcher(
                Box::new(matcher),
                Some(Arc::new(Mutex::new(handler.clone()))),
            );
        } else {
            filter.add_matcher(Box::new(matcher), None);
        }
    }

    let mut buffer = vec![];
    let mut converter = strategy::OutputConverter::new();
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output: Vec<u8> = converter
            .convert(&filter.process(&buffer[..size])?)
            .into_iter()
            .map(|e| e.1)
            .flatten()
            .collect();
        buffer.clear();
        stdout().write_all(&output)?;
    }

    // Input terminated try to hit strategy termination
    let output = converter
        .convert(&filter.terminate()?)
        .into_iter()
        .map(|e| e.1)
        .collect::<Vec<Vec<u8>>>();
    for data in &output {
        stdout().write_all(data)?;
    }

    Ok(())
}
