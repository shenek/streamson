use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use std::{
    collections::HashMap,
    io::{stdin, Read},
    sync::{Arc, Mutex},
};
use streamson_lib::{error, handler, matcher, Collector};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024; // 1MB

fn write_file_validator(input: String) -> Result<(), String> {
    if input.contains(':') {
        Ok(())
    } else {
        Err(format!("{} is not valid input", input))
    }
}

fn usize_validator(input: String) -> Result<(), String> {
    let res = input.parse::<usize>().map_err(|err| err.to_string())?;
    if res == 0 {
        Err("Buffer can't have 0 size".into())
    } else {
        Ok(())
    }
}

fn make_simple_combined_matcher(input: &[&str]) -> Option<matcher::Combinator> {
    input.iter().fold(None, |comb, path| {
        if let Ok(simple) = matcher::Simple::new(path) {
            Some(if let Some(c) = comb {
                c | matcher::Combinator::new(simple)
            } else {
                matcher::Combinator::new(simple)
            })
        } else {
            comb
        }
    })
}

fn main() -> Result<(), error::General> {
    let default_buffer_size = DEFAULT_BUFFER_SIZE.to_string();

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
        )
        .arg(
            Arg::with_name("buffer_size")
                .help("Sets internal buffer size")
                .short("b")
                .long("buffer-size")
                .takes_value(true)
                .validator(usize_validator)
                .value_name("BUFFER_SIZE")
                .default_value(&default_buffer_size)
                .required(false),
        );

    let arg_matches = app.get_matches();

    let mut collector = Collector::new();
    let print_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
    let print_with_header_handler =
        Arc::new(Mutex::new(handler::PrintLn::new().set_use_path(true)));
    let mut file_handler_map: HashMap<String, Arc<Mutex<handler::File>>> = HashMap::new();

    if let Some(simple_matches) = arg_matches.values_of("print") {
        let matcher = make_simple_combined_matcher(&simple_matches.collect::<Vec<&str>>());
        if let Some(matcher) = matcher {
            collector.add_matcher(Box::new(matcher), &[print_handler]);
        }
    }

    if let Some(simple_matches) = arg_matches.values_of("print_with_header") {
        let matcher = make_simple_combined_matcher(&simple_matches.collect::<Vec<&str>>());
        if let Some(matcher) = matcher {
            collector.add_matcher(Box::new(matcher), &[print_with_header_handler]);
        }
    }

    let buffer_size: usize = arg_matches
        .value_of("buffer_size")
        .unwrap()
        .parse()
        .unwrap();

    if let Some(file_matches) = arg_matches.values_of("file") {
        for file in file_matches {
            let splitted: Vec<String> = file.split(':').map(String::from).collect();
            let path = splitted[1..].join(":");
            let matcher = matcher::Simple::new(&splitted[0])?;
            let file_handler = file_handler_map.entry(path.clone()).or_insert_with(|| {
                Arc::new(Mutex::new(handler::File::new(&path).unwrap_or_else(|_| {
                    panic!("Failed to open output file '{}'", path)
                })))
            });
            collector.add_matcher(Box::new(matcher), &[file_handler.clone()]);
        }
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        collector.process(&buffer[..size])?;
        buffer.clear();
    }

    Ok(())
}
