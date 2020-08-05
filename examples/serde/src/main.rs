use serde::{Deserialize, Serialize};
use streamson_lib::{error, handler, matcher, path::Path, Collector};

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
    fn handle(&mut self, _path: &Path, data: &[u8]) -> Result<(), error::Handler> {
        let new_user = serde_json::from_slice(data).map_err(error::Handler::new)?;
        self.users.push(new_user);
        Ok(())
    }
}

fn main() {
    let handler = Arc::new(Mutex::new(UserHandler::default()));
    let matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
    let mut collector = Collector::new();

    collector.add_matcher(Box::new(matcher), &[handler.clone()]);
    collector.process(br#"{"users": [{"firstname": "Carl", "surname": "Streamson"}, {"firstname": "Stream", "surname": "Carlson"}]}"#).unwrap();

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
