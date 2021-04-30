use clap::{Arg, ArgMatches};
use std::{
    collections::{HashMap, HashSet},
    fs,
    str::FromStr,
    sync::{Arc, Mutex},
};

use streamson_lib::{error, handler};

use crate::utils::split_argument;

pub fn handlers_arg() -> Arg<'static> {
    Arg::new("handler")
        .about("Handler which will be triggered on matched data")
        .short('h')
        .group("handlers")
        .multiple(true)
        .value_name("NAME[.GROUP][,OPTION[,OPTION]][:DEFINITION]")
        .takes_value(true)
        .number_of_values(1)
}

pub fn parse_handlers(
    matches: &ArgMatches,
    strategy_name: &str,
) -> Result<HashMap<String, handler::Group>, error::Handler> {
    let mut res: HashMap<String, handler::Group> = HashMap::new();

    if let Some(handlers) = matches.values_of("handler") {
        for handler_str in handlers {
            let (name, group, options, definition) = split_argument(handler_str);

            let new_handler = make_handler(&name, &definition, &options, strategy_name)?;

            let group_handler = if let Some(hndl) = res.remove(&group) {
                hndl + new_handler
            } else {
                new_handler
            };
            res.insert(group, group_handler);
        }
    }

    Ok(res)
}

fn alias_to_handler_name(name_or_alias: &str) -> &str {
    match name_or_alias {
        "a" | "analyser" => "analyser",
        "f" | "file" => "file",
        "d" | "indenter" => "indenter",
        "x" | "regex" => "regex",
        "r" | "replace" => "replace",
        "s" | "shorten" => "shorten",
        "u" | "unstringify" => "unstringify",
        e => e,
    }
}

fn handlers_for_strategy(strategy_name: &str) -> HashSet<&str> {
    let mut res = HashSet::new();
    match strategy_name {
        "all" => {
            res.insert("analyser");
            res.insert("indenter");
        }
        "extract" => {
            res.insert("file");
            // The rests makes sense only if extracted data are strings
            res.insert("regex");
            res.insert("shorten");
            res.insert("unstringify");
        }
        "filter" => {
            // Note that filter strategy should contain at least one
            // file handler to create a sink for other handlers
            res.insert("file");
            // The rests makes sense only if extracted data are strings
            res.insert("regex");
            res.insert("shorten");
            res.insert("unstringify");
        }
        "convert" => {
            res.insert("file");
            // The rests makes sense only if extracted data are strings
            res.insert("regex");
            res.insert("replace");
            res.insert("shorten");
            res.insert("unstringify");
        }
        "trigger" => {
            // Note that filter strategy should contain at least one
            // file handler to create a sink for other handlers
            res.insert("file");
            // The rests makes sense only if extracted data are strings
            res.insert("regex");
            res.insert("shorten");
            res.insert("unstringify");
        }
        _ => unreachable!(),
    }
    res
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
            Arc::new(Mutex::new(handler::Analyser::from_str(handler_string)?))
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
