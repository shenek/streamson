use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::strategy::{self, Strategy};

use crate::{
    docs::{strategies, Element},
    handlers, matchers,
};

pub fn prepare_trigger_subcommand() -> App<'static> {
    App::new("trigger")
        .about(strategies::Trigger.description())
        .arg(matchers::matchers_arg())
        .arg(handlers::handlers_arg("trigger"))
}

pub fn process_trigger(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut trigger = strategy::Trigger::new();

    let hndlrs = handlers::parse_handlers(matches, "trigger")?;

    for (group, matcher) in matchers::parse_matchers(matches)? {
        if let Some(handler) = hndlrs.get(&group) {
            trigger.add_matcher(Box::new(matcher), Arc::new(Mutex::new(handler.clone())));
        }
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        trigger.process(&buffer[..size])?;
        // forward input from stdin to stdout
        stdout().write_all(&buffer[..size])?;
        buffer.clear();
    }
    trigger.terminate()?;

    Ok(())
}
