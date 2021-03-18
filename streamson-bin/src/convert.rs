use std::{
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

pub fn prepare_convert_subcommand() -> App<'static> {
    App::new("convert")
        .about("Converts parts of JSON")
        .arg(matchers::matchers_arg())
        .arg(
            Arg::new("replace")
                .about("Replaces matched part by given string")
                .short('r')
                .group("handler")
                .long("replace")
                .takes_value(true)
                .value_name("JSON")
                .required_unless_present_any(&["shorten", "unstringify", "regex_convert"]),
        )
        .arg(
            Arg::new("shorten")
                .about("Shortens matched data")
                .short('o')
                .group("handler")
                .long("shorten")
                .takes_value(true)
                .value_names(&["LENGTH", "TERMINATOR"])
                .number_of_values(2)
                .required_unless_present_any(&["replace", "unstringify", "regex_convert"]),
        )
        .arg(
            Arg::new("unstringify")
                .about("Unstringifies matched data")
                .short('u')
                .group("handler")
                .long("unstringify")
                .takes_value(false)
                .required_unless_present_any(&["replace", "shorten", "regex_convert"]),
        )
        .arg(
            Arg::new("regex_convert")
                .about("Converts using regex")
                .short('X')
                .group("handler")
                .long("regex-convert")
                .takes_value(true)
                .value_names(&["MATCH", "INTO"])
                .number_of_values(2)
                .required_unless_present_any(&["replace", "shorten", "unstringify"]),
        )
}

pub fn process_convert(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut convert = strategy::Convert::new();

    let mut matchers = matchers::parse_matchers(matches)?;
    let matcher = matchers.remove(&String::new()); // only default for now

    if let Some(replace_string) = matches.value_of("replace") {
        let converter = Arc::new(Mutex::new(handler::Replace::new(
            replace_string.as_bytes().to_vec(),
        )));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), converter);
        }
    } else if let Some(shorten_args) = matches.values_of("shorten") {
        let args: Vec<String> = shorten_args.map(String::from).collect();
        let converter = Arc::new(Mutex::new(handler::Shorten::new(
            args[0].parse::<usize>()?,
            args[1].clone(),
        )));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), converter);
        }
    } else if matches.is_present("unstringify") {
        let converter = Arc::new(Mutex::new(handler::Unstringify::new()));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), converter);
        }
    } else if let Some(regex_convert_args) = matches.values_of("regex_convert") {
        let args: Vec<String> = regex_convert_args.map(String::from).collect();
        let converter = Arc::new(Mutex::new(handler::Regex::new().add_regex(
            regex::Regex::new(&args[0])?,
            args[1].clone(),
            1,
        )));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), converter);
        }
    } else {
        unreachable!();
    };

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

    Ok(())
}
