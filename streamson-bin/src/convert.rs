use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    str::FromStr,
    sync::{Arc, Mutex},
};

use clap::{App, Arg, ArgMatches, SubCommand};
use streamson_lib::{handler, matcher, strategy};

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
            Arg::with_name("replace")
                .help("Replaces matched part by given string")
                .short("r")
                .long("replace")
                .takes_value(true)
                .value_name("JSON")
                .required(true),
        )
}

pub fn process_convert(
    matches: &ArgMatches<'static>,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
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

    let converter = handler::Replace::new(replace_string.as_bytes().to_vec());

    if let Some(matcher_to_add) = matcher {
        convert.add_matcher(
            Box::new(matcher_to_add),
            vec![Arc::new(Mutex::new(converter))],
        );
    }

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
