mod convert;
mod extract;
mod filter;
mod trigger;
mod utils;

use std::{io, process};

use clap::{crate_authors, crate_description, crate_name, crate_version, App};
use streamson_lib::error;

use crate::{
    convert::{prepare_convert_subcommand, process_convert},
    extract::{prepare_extract_subcommand, process_extract},
    filter::{prepare_filter_subcommand, process_filter},
    trigger::{prepare_trigger_subcommand, process_trigger},
};

fn prepare_app() -> App<'static, 'static> {
    App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .subcommand(prepare_extract_subcommand())
        .subcommand(prepare_filter_subcommand())
        .subcommand(prepare_trigger_subcommand())
        .subcommand(prepare_convert_subcommand())
}

fn main() -> Result<(), error::General> {
    let mut app = prepare_app();

    let arg_matches = app.clone().get_matches();
    match arg_matches.subcommand() {
        ("convert", Some(matches)) => process_convert(matches),
        ("extract", Some(matches)) => process_extract(matches),
        ("filter", Some(matches)) => process_filter(matches),
        ("trigger", Some(matches)) => process_trigger(matches),
        _ => {
            app.write_long_help(&mut io::stdout()).unwrap();
            println!();
            process::exit(1);
        }
    }
}
