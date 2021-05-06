use lazy_static::lazy_static;
use std::collections::HashMap;

use super::Element;
use crate::create_doc_element;

create_doc_element!(
    Simple,
    "simple",
    &["simple", "s"],
    Some("[.group]:definition"),
    "Matches data based on `definition`.\n\
    `[]` will match all items in array\n\
    `[1,3-5]` will match second, fourth to sixth item in array\n\
    `{}` will match any key in object\n\
    `?` will match all items in dict or array\n\
    `*` will match all items in dict or array 0 and times\n\
     Example: 'simple:{\"users\"}[]{\"name\"}'"
);
create_doc_element!(
    Depth,
    "depth",
    &["depth", "d"],
    Some("[.group]:from[-to]"),
    "Matches data based on JSON nested level\n\
    `from` minimal level to match (inclusive)\n\
    `to` max level to match (inclusive)\n\
     Example: 'depth:2-3'"
);
create_doc_element!(
    Regex,
    "regex",
    &["regex", "x"],
    Some("[.group]:regex"),
    "Matches data based on regular expression in path\n\
    (similar to simple matcher but uses regexes)\n\
     Example: 'regex:^\\{\"[Uu][Ss][Ee][Rr][Ss]\"\\}$'"
);

lazy_static! {
    pub static ref MAP: HashMap<&'static str, &'static dyn Element> = {
        let mut res: HashMap<&'static str, &'static dyn Element> = HashMap::new();
        res.insert(Simple.as_ref(), &Simple as &dyn Element);
        res.insert(Depth.as_ref(), &Depth as &dyn Element);
        res.insert(Regex.as_ref(), &Regex as &dyn Element);
        res
    };
}
