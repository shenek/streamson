use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    str::FromStr,
};

use clap::{App, Arg, ArgMatches, SubCommand};
use streamson_lib::{matcher, strategy};

pub fn prepare_filter_subcommand() -> App<'static, 'static> {
    SubCommand::with_name("filter")
        .about("Removes matched parts of JSON")
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
}

pub fn process_filter(
    matches: &ArgMatches<'static>,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    let mut filter = strategy::Filter::new();

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
        filter.add_matcher(Box::new(matcher_to_add));
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output = filter.process(&buffer[..size])?;
        buffer.clear();
        stdout().write_all(&output)?;
    }

    Ok(())
}
