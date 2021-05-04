use clap::{Arg, ArgMatches};
use std::{collections::HashMap, str::FromStr};

use streamson_lib::{error, matcher};

use crate::{docs, utils::split_argument};

pub fn matchers_arg() -> Arg<'static> {
    let about = docs::make_docs(&docs::matchers::MAP, None);
    Arg::new("matcher")
        .about("Matches path in JSON")
        .short('m')
        .group("matchers")
        .multiple(true)
        .value_name("NAME[.GROUP][:DEFINITION]")
        .takes_value(true)
        .number_of_values(1)
        .about(Box::leak(Box::new(about)))
}

pub fn parse_matchers(
    matches: &ArgMatches,
) -> Result<HashMap<String, matcher::Combinator>, error::Matcher> {
    let mut res: HashMap<String, matcher::Combinator> = HashMap::new();

    if let Some(matchers) = matches.values_of("matcher") {
        for matcher_str in matchers {
            let (name, group, _, definition) = split_argument(matcher_str);
            let new_matcher = make_matcher(&name, &definition)?;

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
