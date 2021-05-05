pub mod handlers;
pub mod matchers;

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

pub fn make_about(
    map: &HashMap<&'static str, &'static dyn Element>,
    names: Option<&[&str]>,
) -> String {
    if let Some(names) = names {
        names
            .iter()
            .filter_map(|n| map.get(n))
            .map(|e| e.make_doc() + "\n\n")
            .collect::<Vec<String>>()
            .join("")
            + "\n"
    } else {
        map.values()
            .map(|e| e.make_doc() + "\n\n")
            .collect::<Vec<String>>()
            .join("")
            + "\n"
    }
}
