use std::{
    collections::HashMap,
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, Arg, ArgMatches};
use streamson_lib::{
    handler,
    strategy::{self, Strategy},
};

use crate::matchers;

pub fn prepare_trigger_subcommand() -> App<'static> {
    App::new("trigger")
        .about("Triggers command on matched input")
        .arg(
            Arg::new("print")
                .about("Prints matches to stdout separating records by a newline")
                .short('p')
                .long("print")
                .multiple(true)
                .takes_value(true)
                .number_of_values(1)
                .value_names(&["GROUP_NAME"])
                .required(false),
        )
        .arg(
            Arg::new("print_with_header")
                .about("Prints matches to with header to stdout separating records by a newline")
                .short('P')
                .long("print-with-header")
                .multiple(true)
                .takes_value(true)
                .number_of_values(1)
                .value_names(&["GROUP_NAME"])
                .required(false),
        )
        .arg(
            Arg::new("file")
                .about("Writes matches to file separating records by newline")
                .short('f')
                .long("file")
                .multiple(true)
                .takes_value(true)
                .number_of_values(2)
                .value_names(&["GROUP_NAME", "FILE"])
                .required(false),
        )
        .arg(matchers::matchers_arg())
}

pub fn process_trigger(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut trigger = strategy::Trigger::new();

    let mut printing = false; // printing something to stdout
    let mut handlers: HashMap<String, Arc<Mutex<handler::Group>>> = HashMap::new();

    // Prepare print handlers
    if let Some(prints) = matches.values_of("print") {
        for print in prints {
            printing = true;
            handlers
                .entry(print.to_string())
                .or_insert(Arc::new(Mutex::new(handler::Group::new())))
                .lock()
                .unwrap()
                .add_handler_mut(Arc::new(Mutex::new(handler::PrintLn::new())));
        }
    }

    // Prepare print with header handlers
    if let Some(prints) = matches.values_of("print_with_header") {
        for print in prints {
            printing = true;
            handlers
                .entry(print.to_string())
                .or_insert(Arc::new(Mutex::new(handler::Group::new())))
                .lock()
                .unwrap()
                .add_handler_mut(Arc::new(Mutex::new(
                    handler::PrintLn::new().set_use_path(true),
                )));
        }
    }

    // Prepare file handlers
    if let Some(file_matches) = matches.values_of("file") {
        // prepare matchers
        for file in file_matches
            .map(String::from)
            .collect::<Vec<String>>()
            .chunks(2)
        {
            handlers
                .entry(file[0].to_string())
                .or_insert(Arc::new(Mutex::new(handler::Group::new())))
                .lock()
                .unwrap()
                .add_handler_mut(Arc::new(Mutex::new(handler::File::new(&file[1])?)));
        }
    }

    // Prepare matchers
    for (group, matcher) in matchers::parse_matchers(matches)? {
        if let Some(handler) = handlers.get(&group) {
            trigger.add_matcher(Box::new(matcher), handler.clone());
        }
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        trigger.process(&buffer[..size])?;
        // forward input from stdin to stdout
        // only if trigger doesn't print to stdout
        if !printing {
            stdout().write_all(&buffer[..size])?;
        }
        buffer.clear();
    }

    Ok(())
}
