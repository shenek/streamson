use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
};

use clap::{App, Arg, ArgMatches};
use streamson_lib::strategy::{self, Strategy};

use crate::matchers;

pub fn prepare_extract_subcommand() -> App<'static> {
    App::new("extract")
        .about("Passes only matched parts of JSON")
        .arg(matchers::matchers_arg())
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

    let separator = str_to_vec(matches.value_of("separator").unwrap_or(""));
    let before = str_to_vec(matches.value_of("before").unwrap_or(""));
    let after = str_to_vec(matches.value_of("after").unwrap_or(""));

    let mut matchers = matchers::parse_matchers(matches)?;
    let matcher = matchers.remove(&String::new()); // only default for now

    if let Some(matcher_to_add) = matcher {
        extract.add_matcher(Box::new(matcher_to_add), None);
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
    out.write_all(&after)?;

    Ok(())
}
