use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use std::{
    io::{stdin, Read},
    sync::{Arc, Mutex},
};
use streamson_lib::{error, Collector, PrintLn, Simple};

const BUFFER_SIZE: usize = 2048;

fn main() -> Result<(), error::Generic> {
    let app = App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("simple")
                .help("Simple match")
                .short("s")
                .long("simple")
                .multiple(true)
                .takes_value(true)
                .required(false),
        );

    let matches = app.get_matches();

    let mut collector = Collector::new();
    let handler = Arc::new(Mutex::new(PrintLn));

    if let Some(simple_matches) = matches.values_of("simple") {
        for simple in simple_matches {
            let matcher = Simple::new(simple);
            collector = collector.add_matcher(Box::new(matcher), &[handler.clone()]);
        }
    }

    let mut buffer = [0; BUFFER_SIZE];
    while let Ok(size) = stdin().read(&mut buffer[..]) {
        if size == 0 {
            break;
        }
        collector.process(&buffer[..size])?;
    }

    Ok(())
}
