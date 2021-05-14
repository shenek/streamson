use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::strategy::{self, Output, Strategy};

use crate::{
    docs::{strategies, Element},
    handlers, matchers,
};

pub fn prepare_convert_subcommand() -> App<'static> {
    App::new(strategies::Convert.as_ref())
        .visible_aliases(&strategies::Convert.aliases())
        .about(strategies::Convert.description())
        .arg(matchers::matchers_arg())
        .arg(handlers::handlers_arg("convert"))
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
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        for output in convert.process(&buffer[..size])? {
            if let Output::Data(data) = output {
                stdout().write_all(&data)?;
            }
        }
        buffer.clear();
    }

    // Input terminated try to hit strategy termination
    for output in convert.terminate()? {
        if let Output::Data(data) = output {
            stdout().write_all(&data)?;
        }
    }

    Ok(())
}
