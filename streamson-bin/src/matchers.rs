use clap::{Arg, ArgMatches};
use std::{collections::HashMap, str::FromStr};

use streamson_lib::{error, matcher};

pub fn matchers_arg() -> Arg<'static> {
    Arg::new("matcher")
        .about("Matches path in JSON")
        .short('m')
        .group("matchers")
        .multiple(true)
        .value_names(&["NAME[:GROUP]", "DEFINITION"])
        .takes_value(true)
        .number_of_values(2)
}

pub fn parse_matchers(
    matches: &ArgMatches,
) -> Result<HashMap<String, matcher::Combinator>, error::Matcher> {
    let mut res: HashMap<String, matcher::Combinator> = HashMap::new();

    if let Some(matches) = matches.values_of("matcher") {
        for parts in matches.map(String::from).collect::<Vec<String>>().chunks(2) {
            let splitted = parts[0]
                .splitn(2, ':')
                .map(String::from)
                .collect::<Vec<String>>();
            let (name, group) = match splitted.len() {
                1 => (splitted[0].clone(), String::default()),
                2 => (splitted[0].clone(), splitted[1].clone()),
                _ => unreachable!(),
            };

            let new_matcher = make_matcher(&name, &parts[1])?;

            let matcher = if let Some(mtch) = res.remove(&group) {
                mtch | new_matcher
            } else {
                matcher::Combinator::new(new_matcher)
            };
            res.insert(group, matcher);
        }
    }

    Ok(res)
}

pub fn make_matcher(
    matcher_name: &str,
    matcher_string: &str,
) -> Result<matcher::Combinator, error::Matcher> {
    match matcher_name {
        "d" | "depth" => Ok(matcher::Combinator::new(matcher::Depth::from_str(
            matcher_string,
        )?)),
        "s" | "simple" => Ok(matcher::Combinator::new(matcher::Simple::from_str(
            matcher_string,
        )?)),
        "x" | "regex" => Ok(matcher::Combinator::new(matcher::Regex::from_str(
            matcher_string,
        )?)),
        _ => Err(error::Matcher::Parse(format!(
            "Unknown type {}",
            matcher_name
        ))),
    }
}
