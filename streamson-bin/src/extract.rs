use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    str::FromStr,
};

use clap::{App, Arg, ArgMatches};
use streamson_lib::{matcher, strategy};

pub fn prepare_extract_subcommand() -> App<'static> {
    App::new("extract")
        .about("Passes only matched parts of JSON")
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
            Arg::new("separator")
                .about("Separator which will be inserted between matched parts")
                .short('S')
                .long("separator")
                .takes_value(true)
                .value_name("SEP"),
        )
        .arg(
            Arg::new("before")
                .about("Will be print to stdout before first match")
                .short('b')
                .long("before")
                .takes_value(true)
                .value_name("START"),
        )
        .arg(
            Arg::new("after")
                .about("Will be print to stdout after last match")
                .short('a')
                .long("after")
                .takes_value(true)
                .value_name("END"),
        )
}

fn str_to_vec(input: &str) -> Vec<u8> {
    input.as_bytes().iter().copied().collect()
}

pub fn process_extract(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut extract = strategy::Extract::new();

    let mut matcher: Option<matcher::Combinator> = None;

    let separator = str_to_vec(matches.value_of("separator").unwrap_or(""));
    let before = str_to_vec(matches.value_of("before").unwrap_or(""));
    let after = str_to_vec(matches.value_of("after").unwrap_or(""));

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
    let mut buffer = vec![];
    let mut first = true;
    let mut out = stdout();

    out.write_all(&before)?;
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        let output = extract.process(&buffer[..size])?;
        buffer.clear();
        for (_, data) in output {
            if !first && !data.is_empty() {
                out.write_all(&separator)?;
            }
            out.write_all(&data)?;

            first = false;
        }
    }
    out.write_all(&after)?;

    Ok(())
}
