use std::{
    io::{stdin, stdout, Read, Write},
    str::FromStr,
    sync::{Arc, Mutex},
};

use clap::{App, Arg, ArgMatches, SubCommand};
use lazy_static::lazy_static;
use streamson_lib::{error, matcher, strategy, Path};

use crate::utils::usize_validator;

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024; // 1MB
lazy_static! {
    static ref DEFAULT_BUFFER_SIZE_STRING: String = DEFAULT_BUFFER_SIZE.to_string();
    static ref STORED_REPLACE: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
}

pub fn prepare_convert_subcommand() -> App<'static, 'static> {
    SubCommand::with_name("convert")
        .about("Converts parts of JSON")
        .arg(
            Arg::with_name("simple")
                .help("Match by simple match")
                .short("s")
                .long("simple")
                .multiple(true)
                .takes_value(true)
                .value_name("SIMPLE_MATCH")
                .required(false),
        )
        .arg(
            Arg::with_name("depth")
                .help("Match by depth")
                .short("d")
                .long("depth")
                .multiple(true)
                .takes_value(true)
                .value_name("DEPTH_MATCH")
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
                .default_value(&DEFAULT_BUFFER_SIZE_STRING)
                .required(false),
        )
        .arg(
            Arg::with_name("replace")
                .help("Replaces matched part by given string")
                .short("r")
                .long("replace")
                .takes_value(true)
                .value_name("JSON")
                .required(true),
        )
}

pub fn process_convert(matches: &ArgMatches<'static>) -> Result<(), error::General> {
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

    let replace_string = matches.value_of("replace").unwrap();

    // Writing to static
    {
        let mut guard = STORED_REPLACE.lock().unwrap();
        *guard = Some(replace_string.to_string());
    }

    let closure = |_: &Path, _: &[u8]| {
        // Reading from shared static
        STORED_REPLACE
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .as_bytes()
            .iter()
            .copied()
            .collect::<Vec<u8>>()
    };

    if let Some(matcher_to_add) = matcher {
        convert.add_matcher(Box::new(matcher_to_add), Box::new(closure));
    }

    let buffer_size: usize = matches.value_of("buffer_size").unwrap().parse().unwrap();
    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output = convert.process(&buffer[..size])?;
        for data in output {
            stdout().write_all(&data)?;
        }
    }

    Ok(())
}
