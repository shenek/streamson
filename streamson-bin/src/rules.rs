use std::collections::HashSet;

pub fn handlers_for_strategy(strategy_name: &str) -> HashSet<&str> {
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
            // Note that strategy should contain at least one
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
            // Note that strategy should contain at least one
            // file handler to create a sink for other handlers
            res.insert("file");
            // Note that all matches are supposed to be leafs (no array nor object)
            // The rests makes sense only if extracted data are strings
            res.insert("regex");
            res.insert("shorten");
            res.insert("unstringify");
            res.insert("csv");
        }
        _ => unreachable!(),
    }
    res
}
