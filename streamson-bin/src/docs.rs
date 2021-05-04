pub mod handlers;
pub mod matchers;

use std::collections::HashMap;

pub trait Element: AsRef<str> + Sync {
    fn names(&self) -> &[&str];
    fn description(&self) -> &str;
    fn metavar(&self) -> Option<&str>;

    fn make_doc(&self) -> String {
        format!(
            "{} {}\naliases: {}\n{}",
            self.as_ref(),
            self.metavar().unwrap_or(""),
            self.names()
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join(","),
            self.description(),
        )
    }
}

#[macro_export]
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

pub fn make_docs(
    map: &HashMap<&'static str, &'static dyn Element>,
    names: Option<&[&str]>,
) -> String {
    if let Some(names) = names {
        names
            .iter()
            .filter_map(|n| map.get(n))
            .map(|e| e.make_doc())
            .collect::<Vec<String>>()
            .join("\n\n")
    } else {
        map.values()
            .map(|e| e.make_doc())
            .collect::<Vec<String>>()
            .join("\n\n")
    }
}
