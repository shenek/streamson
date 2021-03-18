use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
};

use clap::{App, ArgMatches};
use streamson_lib::strategy::{self, Strategy};

use crate::matchers;

pub fn prepare_filter_subcommand() -> App<'static> {
    App::new("filter")
        .about("Removes matched parts of JSON")
        .arg(matchers::matchers_arg())
}

pub fn process_filter(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut filter = strategy::Filter::new();

    let mut matchers = matchers::parse_matchers(matches)?;
    let matcher = matchers.remove(&String::new()); // only default for now
    if let Some(matcher_to_add) = matcher {
        filter.add_matcher(Box::new(matcher_to_add), None);
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output: Vec<u8> = strategy::OutputConverter::new()
            .convert(&filter.process(&buffer[..size])?)
            .into_iter()
            .map(|e| e.1)
            .flatten()
            .collect();
        buffer.clear();
        stdout().write_all(&output)?;
    }

    Ok(())
}
