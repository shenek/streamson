use std::{
    error::Error,
    io::{stdin, stdout, Read, Write},
    sync::{Arc, Mutex},
};

use clap::{App, ArgMatches};
use streamson_lib::{
    handler::{self, Handler},
    strategy::{self, Output, Strategy},
};

use crate::{
    docs::{strategies, Element},
    handlers,
};

pub fn prepare_all_subcommand() -> App<'static> {
    App::new("all")
        .about(strategies::All.description())
        .arg(handlers::handlers_arg("all"))
}

pub fn process_all(matches: &ArgMatches, buffer_size: usize) -> Result<(), Box<dyn Error>> {
    let mut all = strategy::All::new();

    let hndlrs: Vec<Arc<Mutex<handler::Group>>> = handlers::parse_handlers(matches, "all")?
        .into_iter()
        .map(|(_, handler)| Arc::new(Mutex::new(handler)))
        .collect();

    let converter = hndlrs.iter().any(|e| e.lock().unwrap().is_converter());
    if converter {
        all.set_convert(converter);
    }

    for handler in hndlrs.iter() {
        all.add_handler(handler.clone());
    }

    let mut buffer = vec![];
    while let Ok(size) = stdin().take(buffer_size as u64).read_to_end(&mut buffer) {
        if size == 0 {
            break;
        }

        let output = all.process(&buffer[..size])?;

        if converter {
            for out in output {
                if let Output::Data(data) = out {
                    stdout().write_all(&data)?;
                }
            }
        } else {
            stdout().write_all(&buffer[..size])?;
        }

        buffer.clear();
    }

    if converter {
        // Input terminated try to hit strategy termination
        for out in all.terminate()? {
            if let Output::Data(data) = out {
                stdout().write_all(&data)?;
            }
        }
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
