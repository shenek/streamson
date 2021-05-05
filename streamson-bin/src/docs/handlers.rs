use lazy_static::lazy_static;
use std::collections::HashMap;

use super::Element;
use crate::create_doc_element;

create_doc_element!(
    Analyser,
    "analyser",
    &["analyser", "a"],
    Some("[.group]"),
    "Reads entire JSON and prints structure analysis to stderr"
);
create_doc_element!(
    File,
    "file",
    &["file", "f"],
    Some("[.group][,write_path]:output_file"),
    "Writes matched data to output file.\n\
    If `write_path` is defined in separates output JSON by path.\n\
    Example: 'file:/tmp/output.json'"
);
create_doc_element!(
    Indenter,
    "indenter",
    &["indenter", "i"],
    Some("[.group][:indentation_step]"),
    "Alters intentation of input JSON.\n\
    If indentation_step is not defined it will produce compressed JSON,\n\
    otherwise it represents number of spaces in the output.\n\
    Example: 'indenter:2'"
);
create_doc_element!(
    Regex,
    "regex",
    &["regex", "x"],
    Some("[.group][:regex]"),
    "Uses sed regex to convert matched output.\n\
    Example: 'regex:s/user/User'"
);
create_doc_element!(
    Replace,
    "replace",
    &["replace", "r"],
    Some("[.group]:replacement"),
    "Replaces matched data by other data.\n\
     `replacement` sets data to which the matched output is\n\
     going to be replaced.\n\
     Example: 'replace:null'"
);
create_doc_element!(
    Shorten,
    "shorten",
    &["shorten", "s"],
    Some("[.group]:char_count,terminator"),
    "Makes the matched data shorter.\n\
     `char_count` - max length\n\
     `terminator` - if max length is reached these\n\
     chars will be inserted\n\
     Example: 'shorten:3,..\"'"
);
create_doc_element!(
    Unstringify,
    "unstringify",
    &["unstringify", "u"],
    Some("[.group]"),
    "Expects a string on the input and\n\
     converts it to data e.g. '\"null\"' -> null"
);

lazy_static! {
    pub static ref MAP: HashMap<&'static str, &'static dyn Element> = {
        let mut res: HashMap<&'static str, &'static dyn Element> = HashMap::new();
        res.insert(Analyser.as_ref(), &Analyser as &dyn Element);
        res.insert(File.as_ref(), &File as &dyn Element);
        res.insert(Indenter.as_ref(), &Indenter as &dyn Element);
        res.insert(Regex.as_ref(), &Regex as &dyn Element);
        res.insert(Replace.as_ref(), &Replace as &dyn Element);
        res.insert(Shorten.as_ref(), &Shorten as &dyn Element);
        res.insert(Unstringify.as_ref(), &Unstringify as &dyn Element);
        res
    };
}
