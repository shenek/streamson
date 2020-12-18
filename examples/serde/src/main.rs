use serde::{Deserialize, Serialize};
use streamson_lib::{error, handler, matcher, path::Path, strategy, streamer::ParsedKind};

use std::sync::{Arc, Mutex};

/// User instance
#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    firstname: String,
    surname: String,
}

impl User {
    pub fn say_your_name(&self) -> String {
        format!("My name is {} {} !!!", self.firstname, self.surname)
    }
}

/// Custom handler which collects users
#[derive(Debug, Default)]
pub struct UserHandler {
    pub users: Vec<User>,
}

impl handler::Handler for UserHandler {
    fn handle(
        &mut self,
        _path: &Path,
        _match_idx: usize,
        data: Option<&[u8]>,
        _kind: ParsedKind,
    ) -> Result<Option<Vec<u8>>, error::Handler> {
        let new_user = serde_json::from_slice(data.unwrap()).map_err(error::Handler::new)?;
        self.users.push(new_user);
        Ok(None)
    }
}

fn main() {
    let handler = Arc::new(Mutex::new(UserHandler::default()));
    let matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let mut trigger = strategy::Trigger::new();

    trigger.add_matcher(Box::new(matcher), &[handler.clone()]);
    trigger.process(br#"{"users": [{"firstname": "Carl", "surname": "Streamson"}, {"firstname": "Stream", "surname": "Carlson"}]}"#).unwrap();

    handler
        .lock()
        .unwrap()
        .users
        .iter()
        .enumerate()
        .for_each(|(idx, user)| println!("user{}: {}", idx, user.say_your_name()));

    // should print
    // user0: My name is Carl Streamson !!!
    // user1: My name is Stream Carlson !!!
}
