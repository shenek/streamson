use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::{
    handler,
    strategy::{self, Strategy},
};

use crate::handlers;

pub fn prepare_all_subcommand() -> App<'static> {
    App::new("all")
        .about("Strategy which matches all elements (no need to set matchers)")
        .arg(handlers::handlers_arg())
}

pub fn process_all(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut all = strategy::All::new();

    let hndlrs: Vec<Arc<Mutex<handler::Group>>> = handlers::parse_handlers(matches)?
        .into_iter()
        .map(|(_, handler)| Arc::new(Mutex::new(handler)))
        .collect();

    for handler in hndlrs.iter() {
        all.add_handler(handler.clone());
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }
        all.process(&buffer[..size])?;
        stdout().write_all(&buffer[..size])?;
    }

    for handler in hndlrs.clone() {
        for sub_handler in handler.lock().unwrap().subhandlers() {
            if let Some(analyser) = sub_handler
                .lock()
                .unwrap()
                .as_any()
                .downcast_ref::<handler::Analyser>()
            {
                eprintln!("JSON structure:");
                for (path, count) in analyser.results() {
                    eprintln!(
                        "  {}: {}",
                        if path.is_empty() { "<root>" } else { &path },
                        count
                    );
                }
            }
        }
    }
    all.terminate()?;

    Ok(())
}
