use std::collections::HashMap;

pub trait Element: AsRef<str> + Sync {
    fn names(&self) -> &[&str];
    fn description(&self) -> &str;
    fn metavar(&self) -> Option<&str>;
    fn idented_description(&self) -> String {
        let mut output = "  ".to_string();
        for chr in self.description().chars() {
            output += &chr.to_string();
            if chr == '\n' {
                output += "  ";
            }
        }
        output
    }

    fn make_doc(&self) -> String {
        format!(
            "({}){}\n{}",
            self.names()
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join("|"),
            self.metavar().unwrap_or(""),
            self.idented_description(),
        )
    }

    #[cfg(feature = "man")]
    fn extend_man_section(&self, section: man::Section) -> man::Section {
        section
            .paragraph(&format!(
                "({}){}",
                self.names()
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join("|"),
                self.metavar().unwrap_or(""),
            ))
            .paragraph(&self.idented_description())
    }
}

macro_rules! create_doc_element {
    ($classname:ident, $name:literal, $names:expr, $metavar:expr, $description:expr) => {
        pub struct $classname;

        impl AsRef<str> for $classname {
            fn as_ref(&self) -> &str {
                $name
            }
        }

        impl Element for $classname {
            fn names(&self) -> &[&str] {
                $names
            }

            fn description(&self) -> &str {
                $description
            }

            fn metavar(&self) -> Option<&str> {
                $metavar
            }
        }
    };
}

pub fn make_about(
    map: &HashMap<&'static str, &'static dyn Element>,
    names: Option<&[&str]>,
) -> String {
    let element_names = if let Some(names) = names {
        let mut names: Vec<&str> = names.to_vec();
        names.sort_unstable();
        names
    } else {
        let mut keys: Vec<&str> = map.keys().copied().collect();
        keys.sort_unstable();
        keys
    };
    element_names
        .iter()
        .filter_map(|n| map.get(n))
        .map(|e| e.make_doc() + "\n\n")
        .collect::<Vec<String>>()
        .join("")
        + "\n"
}

#[cfg(feature = "man")]
#[allow(dead_code)] // it is used in build.rs
pub fn make_man_section(
    map: &HashMap<&'static str, &'static dyn Element>,
    names: Option<&[&str]>,
    section_name: &str,
) -> man::Section {
    let element_names = if let Some(names) = names {
        let mut names: Vec<&str> = names.to_vec();
        names.sort_unstable();
        names
    } else {
        let mut keys: Vec<&str> = map.keys().copied().collect();
        keys.sort_unstable();
        keys
    };

    let section = man::Section::new(section_name);

    element_names
        .into_iter()
        .filter_map(|n| map.get(n))
        .fold(section, |section, e| e.extend_man_section(section))
}

pub mod handlers {
    use super::Element;
    use lazy_static::lazy_static;
    use std::collections::HashMap;

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
}

pub mod matchers {
    use super::Element;
    use lazy_static::lazy_static;
    use std::collections::HashMap;

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
}

pub mod strategies {
    use super::Element;
    use lazy_static::lazy_static;
    use std::collections::HashMap;

    create_doc_element!(
        All,
        "all",
        &["all"],
        None,
        "Strategy which matches all elements (no need to set matchers)"
    );

    create_doc_element!(
        Convert,
        "convert",
        &["convert"],
        None,
        "Converts parts of JSON"
    );

    create_doc_element!(
        Extract,
        "extract",
        &["extract"],
        None,
        "Passes only matched parts of JSON"
    );

    create_doc_element!(
        Filter,
        "filter",
        &["filter"],
        None,
        "Removes matched parts of JSON"
    );

    create_doc_element!(
        Trigger,
        "trigger",
        &["trigger"],
        None,
        "Triggers command on matched input"
    );

    lazy_static! {
        pub static ref MAP: HashMap<&'static str, &'static dyn Element> = {
            let mut res: HashMap<&'static str, &'static dyn Element> = HashMap::new();
            res.insert(All.as_ref(), &All as &dyn Element);
            res.insert(Convert.as_ref(), &Convert as &dyn Element);
            res.insert(Extract.as_ref(), &Extract as &dyn Element);
            res.insert(Filter.as_ref(), &Filter as &dyn Element);
            res.insert(Trigger.as_ref(), &Trigger as &dyn Element);
            res
        };
    }
}
