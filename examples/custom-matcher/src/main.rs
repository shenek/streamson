use streamson_lib::{handler, matcher, path::Path, strategy, streamer::ParsedKind};

use std::sync::{Arc, Mutex};

/// A custom matcher which matches the path which contains the
/// letter
#[derive(Default, Debug)]
pub struct Letter {
    letter: char,
}

impl Letter {
    /// Creates a new instance of letter matcher
    pub fn new(letter: char) -> Self {
        Self { letter }
    }
}

impl matcher::MatchMaker for Letter {
    fn match_path(&self, path: &Path, _kind: ParsedKind) -> bool {
        path.to_string().chars().any(|c| c == self.letter)
    }
}

fn main() {
    let handler = Arc::new(Mutex::new(handler::PrintLn::new().set_use_path(true)));
    let matcher = Letter::new('l');
    let mut trigger = strategy::Trigger::new();

    trigger.add_matcher(Box::new(matcher), handler);
    trigger
        .process(br#"{"first": {"log": [1,2,3,4]}}"#)
        .unwrap();

    // should print
    //
    // {"first"}{"log"}[0]: 1
    // {"first"}{"log"}[1]: 2
    // {"first"}{"log"}[2]: 3
    // {"first"}{"log"}[3]: 4
    // {"first"}{"log"}: [1,2,3,4]
    //
    // because given paths contain 'l' letter
}
