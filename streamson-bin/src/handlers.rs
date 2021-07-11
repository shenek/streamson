use clap::{Arg, ArgMatches};
use std::{
    collections::HashMap,
    fs,
    str::FromStr,
    sync::{Arc, Mutex},
};

use streamson_lib::{error, handler};

use crate::{docs, rules::handlers_for_strategy, utils::split_argument};

pub fn handlers_arg(strategy_name: &str) -> Arg<'static> {
    let handler_names = handlers_for_strategy(strategy_name);
    let about = docs::make_about(
        &docs::handlers::MAP,
        Some(&handler_names.into_iter().collect::<Vec<&str>>()),
    );
    Arg::new("handler")
        .about("Handler which will be triggered on matched data")
        .short('h')
        .group("handlers")
        .multiple(true)
        .takes_value(true)
        .number_of_values(1)
        .about(Box::leak(Box::new(about)))
}

pub fn parse_handlers(
    matches: &ArgMatches,
    strategy_name: &str,
) -> Result<HashMap<String, handler::Group>, error::Handler> {
    let mut res: HashMap<String, handler::Group> = HashMap::new();

    if let Some(handlers) = matches.values_of("handler") {
        for handler_str in handlers {
            let (name, groups, options, definition) = split_argument(handler_str);

            let new_handler = make_handler(&name, &definition, &options, strategy_name)?;

            for group in groups {
                let group_handler = if let Some(hndl) = res.remove(&group) {
                    hndl + new_handler.clone()
                } else {
                    new_handler.clone()
                };
                res.insert(group, group_handler);
            }
        }
    }

    Ok(res)
}

fn alias_to_handler_name(name_or_alias: &str) -> &str {
    match name_or_alias {
        "a" | "analyser" => "analyser",
        "c" | "csv" => "csv",
        "f" | "file" => "file",
        "d" | "indenter" => "indenter",
        "x" | "regex" => "regex",
        "r" | "replace" => "replace",
        "s" | "shorten" => "shorten",
        "u" | "unstringify" => "unstringify",
        e => e,
    }
}

pub fn make_handler(
    handler_name: &str,
    handler_string: &str,
    options: &[String],
    strategy_name: &str,
) -> Result<handler::Group, error::Handler> {
    let real_name = alias_to_handler_name(handler_name);

    if !handlers_for_strategy(strategy_name).contains(real_name) {
        return Err(error::Handler::new(format!(
            "handler `{}` can not be used in `{}` strategy.",
            handler_name, strategy_name
        )));
    }

    let wrong_number_of_options_error = error::Handler::new(format!(
        "Wrong file handler options number {}",
        options.len()
    ));

    let inner: Arc<Mutex<dyn handler::Handler>> = match real_name {
        "analyser" => {
            if !options.is_empty() {
                return Err(wrong_number_of_options_error);
            }
            let mut analyser = handler::Analyser::from_str(handler_string)?;
            analyser.set_input_finished_callback(Some(Box::new(|analyser| {
                eprintln!("JSON structure:");
                for (path, count) in analyser.results() {
                    eprintln!(
                        "  {}: {}",
                        if path.is_empty() { "<root>" } else { &path },
                        count
                    );
                }
            })));
            Arc::new(Mutex::new(analyser))
        }
        "csv" => {
            let csv = handler::Csv::from_str(handler_string)?;
            Arc::new(Mutex::new(csv))
        }
        "file" => {
            if options.len() > 1 {
                return Err(wrong_number_of_options_error);
            }
            let mut handler = handler::Output::<fs::File>::from_str(handler_string)?;
            if !options.is_empty() {
                let write_path: bool = options[0].parse().map_err(error::Handler::new)?;
                handler = handler.set_write_path(write_path);
            }
            // print path option
            Arc::new(Mutex::new(handler))
        }
        "indenter" => {
            if !options.is_empty() {
                return Err(wrong_number_of_options_error);
            }
            Arc::new(Mutex::new(handler::Indenter::from_str(handler_string)?))
        }
        "regex" => {
            if !options.is_empty() {
                return Err(wrong_number_of_options_error);
            }
            Arc::new(Mutex::new(handler::Regex::from_str(handler_string)?))
        }
        "replace" => {
            if !options.is_empty() {
                return Err(wrong_number_of_options_error);
            }
            Arc::new(Mutex::new(handler::Replace::from_str(handler_string)?))
        }
        "shorten" => {
            if !options.is_empty() {
                return Err(wrong_number_of_options_error);
            }
            Arc::new(Mutex::new(handler::Shorten::from_str(handler_string)?))
        }
        "unstringify" => {
            if !options.is_empty() {
                return Err(wrong_number_of_options_error);
            }
            Arc::new(Mutex::new(handler::Unstringify::from_str(handler_string)?))
        }
        _ => {
            return Err(error::Handler::new(format!(
                "Unknown handler type {}",
                handler_name
            )))
        }
    };

    Ok(handler::Group::new().add_handler(inner))
}
