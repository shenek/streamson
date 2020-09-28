use std::{
    io::{stdin, stdout, Read, Write},
    str::FromStr,
};

use clap::{App, Arg, ArgMatches, SubCommand};
use lazy_static::lazy_static;
use streamson_lib::{error, matcher, strategy};

use crate::utils::usize_validator;

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024; // 1MB
lazy_static! {
    static ref DEFAULT_BUFFER_SIZE_STRING: String = DEFAULT_BUFFER_SIZE.to_string();
}

pub fn prepare_extract_subcommand() -> App<'static, 'static> {
    SubCommand::with_name("extract")
        .about("Passes only matched parts of JSON")
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
}

pub fn process_extract(matches: &ArgMatches<'static>) -> Result<(), error::General> {
    let mut extract = strategy::Extract::new();

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

    if let Some(matcher_to_add) = matcher {
        extract.add_matcher(Box::new(matcher_to_add));
    }

    let buffer_size: usize = matches.value_of("buffer_size").unwrap().parse().unwrap();
    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let (output, end) = extract.process(&buffer[..size])?;
        for (_, data) in output {
            stdout().write_all(&data)?;
        }

        // No more output
        if end {
            break;
        }
    }

    Ok(())
}
