use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::strategy::{self, Output, Strategy};

use crate::{handlers, matchers};

pub fn prepare_filter_subcommand() -> App<'static> {
    App::new("filter")
        .about("Removes matched parts of JSON")
        .arg(matchers::matchers_arg())
        .arg(handlers::handlers_arg())
}

pub fn process_filter(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut filter = strategy::Filter::new();

    let hndlrs = handlers::parse_handlers(matches, "filter")?;

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
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }

        for output in filter.process(&buffer[..size])? {
            if let Output::Data(data) = output {
                stdout().write_all(&data)?;
            }
        }
        buffer.clear();
    }

    // Input terminated try to hit strategy termination
    for output in filter.terminate()? {
        if let Output::Data(data) = output {
            stdout().write_all(&data)?;
        }
    }

    Ok(())
}
