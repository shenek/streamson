use clap::{Arg, ArgMatches};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use streamson_lib::{error, handler};

pub fn handlers_arg() -> Arg<'static> {
    Arg::new("handler")
        .about("Handler which will be triggered on matched data")
        .short('h')
        .group("handlers")
        .multiple(true)
        .value_names(&["NAME[:GROUP]", "DEFINITION"])
        .takes_value(true)
        .number_of_values(2)
}

pub fn parse_handlers(
    matches: &ArgMatches,
) -> Result<HashMap<String, handler::Group>, error::Handler> {
    let mut res: HashMap<String, handler::Group> = HashMap::new();

    if let Some(handlers) = matches.values_of("handler") {
        for parts in handlers
            .map(String::from)
            .collect::<Vec<String>>()
            .chunks(2)
        {
            let splitted = parts[0]
                .splitn(2, ':')
                .map(String::from)
                .collect::<Vec<String>>();
            let (name, group) = match splitted.len() {
                1 => (splitted[0].clone(), String::default()),
                2 => (splitted[0].clone(), splitted[1].clone()),
                _ => unreachable!(),
            };

            let new_handler = make_handler(&name, &parts[1])?;

            let group_handler = if let Some(hndl) = res.remove(&group) {
                hndl.add_handler(new_handler)
            } else {
                handler::Group::new().add_handler(new_handler)
            };
            res.insert(group, group_handler);
        }
    }

    Ok(res)
}

pub fn make_handler(
    handler_name: &str,
    handler_string: &str,
) -> Result<Arc<Mutex<dyn handler::Handler>>, error::Handler> {
    match handler_name {
        "a" | "analyser" => Ok(Arc::new(Mutex::new(handler::Analyser::from_str(
            handler_string,
        )?))),
        "f" | "file" => Ok(Arc::new(Mutex::new(handler::File::from_str(
            handler_string,
        )?))),
        "d" | "indenter" => Ok(Arc::new(Mutex::new(handler::Indenter::from_str(
            handler_string,
        )?))),
        "p" | "println" => Ok(Arc::new(Mutex::new(handler::PrintLn::from_str(
            handler_string,
        )?))),
        "x" | "regex" => Ok(Arc::new(Mutex::new(handler::Regex::from_str(
            handler_string,
        )?))),
        "r" | "replace" => Ok(Arc::new(Mutex::new(handler::Replace::from_str(
            handler_string,
        )?))),
        "s" | "shorten" => Ok(Arc::new(Mutex::new(handler::Shorten::from_str(
            handler_string,
        )?))),
        "u" | "unstringify" => Ok(Arc::new(Mutex::new(handler::Shorten::from_str(
            handler_string,
        )?))),
        _ => Err(error::Handler::new(format!(
            "Unknown handler type {}",
            handler_name
        ))),
    }
}
