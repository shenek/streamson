mod trigger;

use std::{io, process};

use clap::{crate_authors, crate_description, crate_name, crate_version, App};
use streamson_lib::error;

use crate::trigger::{prepare_trigger_subcommand, process_trigger};

fn prepare_app() -> App<'static, 'static> {
    App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .subcommand(prepare_trigger_subcommand())
}

fn main() -> Result<(), error::General> {
    let mut app = prepare_app();

    let arg_matches = app.clone().get_matches();
    match arg_matches.subcommand() {
        ("trigger", Some(matches)) => process_trigger(matches),
        _ => {
            app.write_long_help(&mut io::stdout()).unwrap();
            println!();
            process::exit(1);
        }
    }
}
