use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    str::FromStr,
    sync::{Arc, Mutex},
};

use clap::{App, Arg, ArgMatches};
use streamson_lib::{handler, matcher, strategy};

pub fn prepare_convert_subcommand() -> App<'static> {
    App::new("convert")
        .about("Converts parts of JSON")
        .arg(
            Arg::new("simple")
                .about("Match by simple match")
                .short('s')
                .long("simple")
                .multiple(true)
                .takes_value(true)
                .value_name("SIMPLE_MATCH")
                .required(false),
        )
        .arg(
            Arg::new("depth")
                .about("Match by depth")
                .short('d')
                .long("depth")
                .multiple(true)
                .takes_value(true)
                .value_name("DEPTH_MATCH")
                .required(false),
        )
        .arg(
            Arg::new("replace")
                .about("Replaces matched part by given string")
                .short('r')
                .group("handler")
                .long("replace")
                .takes_value(true)
                .value_name("JSON")
                .required_unless_present_any(&["shorten", "unstringify"]),
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
                .required_unless_present_any(&["replace", "unstringify"]),
        )
        .arg(
            Arg::new("unstringify")
                .about("Unstringifies matched data")
                .short('u')
                .group("handler")
                .long("unstringify")
                .takes_value(false)
                .required_unless_present_any(&["replace", "shorten"]),
        )
}

pub fn process_convert(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut convert = strategy::Convert::new();

    let mut matcher: Option<matcher::Combinator> = None;

    if let Some(matches) = matches.values_of("simple") {
        for matcher_str in matches {
            if let Some(old_matcher) = matcher {
                matcher = Some(
                    old_matcher | matcher::Combinator::new(matcher::Simple::new(matcher_str)?),
                );
            } else {
                matcher = Some(matcher::Combinator::new(matcher::Simple::new(matcher_str)?));
            }
        }
    }

    if let Some(matches) = matches.values_of("depth") {
        for matcher_str in matches {
            if let Some(old_matcher) = matcher {
                matcher = Some(
                    old_matcher | matcher::Combinator::new(matcher::Depth::from_str(matcher_str)?),
                );
            } else {
                matcher = Some(matcher::Combinator::new(matcher::Depth::from_str(
                    matcher_str,
                )?));
            }
        }
    }

    if let Some(replace_string) = matches.value_of("replace") {
        let converter = Arc::new(Mutex::new(handler::Replace::new(
            replace_string.as_bytes().to_vec(),
        )));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), vec![converter]);
        }
    } else if let Some(shorten_args) = matches.values_of("shorten") {
        let args: Vec<String> = shorten_args.map(String::from).collect();
        let converter = Arc::new(Mutex::new(handler::Shorten::new(
            args[0].parse::<usize>()?,
            args[1].clone(),
        )));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), vec![converter]);
        }
    } else if matches.is_present("unstringify") {
        let converter = Arc::new(Mutex::new(handler::Unstringify::new()));
        if let Some(matcher_to_add) = matcher {
            convert.add_matcher(Box::new(matcher_to_add), vec![converter]);
        }
    } else {
        unreachable!();
    };

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output = convert.process(&buffer[..size])?;
        buffer.clear();
        for data in output {
            stdout().write_all(&data)?;
        }
    }

    Ok(())
}
