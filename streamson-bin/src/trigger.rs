use std::{
    collections::HashMap,
    error::Error,
    io::{stdin, stdout, Read, Write},
    str::FromStr,
    sync::{Arc, Mutex},
};

use clap::{App, Arg, ArgMatches, SubCommand};
use streamson_lib::{error, handler, matcher, strategy};

fn make_matcher(
    matcher_name: &str,
    matcher_string: &str,
) -> Result<matcher::Combinator, Box<dyn Error>> {
    match matcher_name {
        "depth" => Ok(matcher::Combinator::new(matcher::Depth::from_str(
            matcher_string,
        )?)),
        "simple" => Ok(matcher::Combinator::new(matcher::Simple::from_str(
            matcher_string,
        )?)),
        _ => Err(Box::new(error::Matcher::Parse(format!(
            "Unknown type {}",
            matcher_name
        )))),
    }
}

pub fn prepare_trigger_subcommand() -> App<'static, 'static> {
    SubCommand::with_name("trigger")
        .about("Triggers command on matched input")
        .arg(
            Arg::with_name("print")
                .help("Prints matches to stdout separating records by a newline")
                .short("p")
                .long("print")
                .multiple(true)
                .takes_value(true)
                .number_of_values(2)
                .value_names(&["MATCHER_NAME", "MATCH"])
                .required(false),
        )
        .arg(
            Arg::with_name("print_with_header")
                .help("Prints matches to with header to stdout separating records by a newline")
                .short("P")
                .long("print-with-header")
                .multiple(true)
                .takes_value(true)
                .number_of_values(2)
                .value_names(&["MATCHER_NAME", "MATCH"])
                .required(false),
        )
        .arg(
            Arg::with_name("file")
                .help("Writes matches to file separating records by newline")
                .short("f")
                .long("file")
                .multiple(true)
                .takes_value(true)
                .number_of_values(3)
                .value_names(&["MATCHER_NAME", "MATCH", "FILE"])
                .required(false),
        )
        .arg(
            Arg::with_name("struct")
                .help("Goes through a json and prints JSON structure at the end of processing.")
                .short("s")
                .long("struct")
                .takes_value(false)
                .required(false),
        )
}

fn prepare_matcher_from_list(input: Vec<String>) -> Result<matcher::Combinator, Box<dyn Error>> {
    Ok(input
        .chunks(2)
        .map(|parts| make_matcher(&parts[0], &parts[1]))
        .collect::<Result<Vec<matcher::Combinator>, Box<dyn Error>>>()?
        .into_iter()
        .fold(None, |res, new| {
            if let Some(cmb) = res {
                Some(cmb | new)
            } else {
                Some(new)
            }
        })
        .unwrap())
}

pub fn process_trigger(
    matches: &ArgMatches<'static>,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    let mut trigger = strategy::Trigger::new();
    let print_handler = Arc::new(Mutex::new(handler::PrintLn::new()));
    let print_with_header_handler =
        Arc::new(Mutex::new(handler::PrintLn::new().set_use_path(true)));

    let mut printing = false; // printing something to stdout

    if let Some(simple_matches) = matches.values_of("print") {
        printing = true;
        let matcher = prepare_matcher_from_list(simple_matches.map(String::from).collect())?;
        trigger.add_matcher(Box::new(matcher), &[print_handler]);
    }

    if let Some(simple_matches) = matches.values_of("print_with_header") {
        printing = true;
        let matcher = prepare_matcher_from_list(simple_matches.map(String::from).collect())?;
        trigger.add_matcher(Box::new(matcher), &[print_with_header_handler]);
    }

    if let Some(file_matches) = matches.values_of("file") {
        let mut file_handler_map: HashMap<String, matcher::Combinator> = HashMap::new();

        // prepare matchers
        for parts in file_matches
            .map(String::from)
            .collect::<Vec<String>>()
            .chunks(3)
        {
            let new_matcher = make_matcher(&parts[0], &parts[1])?;
            let matcher = if let Some(matcher) = file_handler_map.remove(&parts[2]) {
                matcher | new_matcher
            } else {
                new_matcher
            };
            file_handler_map.insert(parts[2].clone(), matcher);
        }

        // prepare handlers
        for (filename, matcher) in file_handler_map.into_iter() {
            let handler = Arc::new(Mutex::new(handler::File::new(&filename)?));
            trigger.add_matcher(Box::new(matcher), &[handler]);
        }
    }

    let analyser_handler = if matches.is_present("struct") {
        let matcher = matcher::All::default();
        let handler = Arc::new(Mutex::new(handler::Analyser::new()));
        trigger.add_matcher(Box::new(matcher), &[handler.clone()]);

        Some(handler)
    } else {
        None
    };

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        trigger.process(&buffer[..size])?;
        // forward input from stdin to stderr
        // only if trigger doesn't print to stdout
        if !printing {
            stdout().write_all(&buffer[..size])?;
        }
        buffer.clear();
    }

    if let Some(analyser_handler) = analyser_handler {
        println!("JSON structure:");
        for (path, count) in analyser_handler.lock().unwrap().results() {
            println!(
                "  {}: {}",
                if path.is_empty() { "<root>" } else { &path },
                count
            );
        }
    }

    Ok(())
}
