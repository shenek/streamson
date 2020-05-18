use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use std::{
    collections::HashMap,
    io::{stdin, Read},
    sync::{Arc, Mutex},
};
use streamson_lib::{error, handler, Collector, Simple};

const BUFFER_SIZE: usize = 2048;

fn write_file_validator(input: String) -> Result<(), String> {
    if input.contains(':') {
        Ok(())
    } else {
        Err(format!("{} is not valid input", input))
    }
}

fn main() -> Result<(), error::Generic> {
    let app = App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("print")
                .help("Prints matches to stdout separating records by a newline")
                .short("p")
                .long("print")
                .multiple(true)
                .takes_value(true)
                .value_name("SIMPLE_MATCH")
                .required(false),
        )
        .arg(
            Arg::with_name("print_with_header")
                .help("Prints matches to with header to stdout separating records by a newline")
                .short("P")
                .long("print-with-header")
                .multiple(true)
                .takes_value(true)
                .value_name("SIMPLE_MATCH")
                .required(false),
        )
        .arg(
            Arg::with_name("file")
                .help("Writes matches to file separating records by newline")
                .short("f")
                .long("file")
                .multiple(true)
                .takes_value(true)
                .validator(write_file_validator)
                .value_name("SIMPLE_MATCH:PATH_TO_FILE")
                .required(false),
        );

    let arg_matches = app.get_matches();

    let mut collector = Collector::new();
    let print_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
    let print_with_header_handler =
        Arc::new(Mutex::new(handler::PrintLn::new().set_show_path(true)));
    let mut file_handler_map: HashMap<String, Arc<Mutex<handler::File>>> = HashMap::new();

    if let Some(simple_matches) = arg_matches.values_of("print") {
        for simple in simple_matches {
            let matcher = Simple::new(simple);
            collector = collector.add_matcher(Box::new(matcher), &[print_handler.clone()]);
        }
    }

    if let Some(simple_matches) = arg_matches.values_of("print_with_header") {
        for simple in simple_matches {
            let matcher = Simple::new(simple);
            collector =
                collector.add_matcher(Box::new(matcher), &[print_with_header_handler.clone()]);
        }
    }

    if let Some(file_matches) = arg_matches.values_of("file") {
        for file in file_matches {
            let splitted: Vec<String> = file.split(':').map(String::from).collect();
            let path = splitted[1..].join(":");
            let matcher = Simple::new(&splitted[0]);
            let file_handler = file_handler_map.entry(path.clone()).or_insert_with(|| {
                Arc::new(Mutex::new(handler::File::new(&path).unwrap_or_else(|_| {
                    panic!("Failed to open output file '{}'", path)
                })))
            });
            collector = collector.add_matcher(Box::new(matcher), &[file_handler.clone()]);
        }
    }

    let mut buffer = [0; BUFFER_SIZE];
    while let Ok(size) = stdin().read(&mut buffer[..]) {
        if size == 0 {
            break;
        }
        collector.process(&buffer[..size])?;
    }

    Ok(())
}
