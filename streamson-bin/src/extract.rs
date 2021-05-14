use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, Arg, ArgMatches};
use streamson_lib::strategy::{self, Output, Strategy};

use crate::{
    docs::{strategies, Element},
    handlers, matchers,
};

pub fn prepare_extract_subcommand() -> App<'static> {
    App::new(strategies::Extract.as_ref())
        .visible_aliases(&strategies::Extract.aliases())
        .about(strategies::Extract.description())
        .arg(matchers::matchers_arg())
        .arg(handlers::handlers_arg("extract"))
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
                .about("Will be printed to stdout before first match")
                .short('b')
                .long("before")
                .takes_value(true)
                .value_name("START"),
        )
        .arg(
            Arg::new("after")
                .about("Will be printed to stdout after last match")
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

    let separator = str_to_vec(matches.value_of("separator").unwrap_or(""));
    let before = str_to_vec(matches.value_of("before").unwrap_or(""));
    let after = str_to_vec(matches.value_of("after").unwrap_or(""));

    let hndlrs = handlers::parse_handlers(matches, "extract")?;

    for (group, matcher) in matchers::parse_matchers(matches)? {
        if let Some(handler) = hndlrs.get(&group) {
            extract.add_matcher(
                Box::new(matcher),
                Some(Arc::new(Mutex::new(handler.clone()))),
            );
        } else {
            extract.add_matcher(Box::new(matcher), None);
        }
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
        for part in output {
            match part {
                strategy::Output::Start(_) => {
                    if !first {
                        out.write_all(&separator)?;
                    } else {
                        first = false;
                    }
                }
                strategy::Output::Data(data) => {
                    out.write_all(&data)?;
                }
                strategy::Output::End => {}
            }
        }
    }

    // Input terminated try to hit strategy termination
    for output in extract.terminate()? {
        if let Output::Data(data) = output {
            stdout().write_all(&data)?;
        }
    }

    out.write_all(&after)?;

    Ok(())
}
