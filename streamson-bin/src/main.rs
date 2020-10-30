mod convert;
mod extract;
mod filter;
mod trigger;
mod utils;

use std::{error::Error, io, process};

use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use clap_generate::generators::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_generate::{generate, Generator};
use lazy_static::lazy_static;

use crate::{
    convert::{prepare_convert_subcommand, process_convert},
    extract::{prepare_extract_subcommand, process_extract},
    filter::{prepare_filter_subcommand, process_filter},
    trigger::{prepare_trigger_subcommand, process_trigger},
    utils::usize_validator,
};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024; // 1MB
lazy_static! {
    static ref DEFAULT_BUFFER_SIZE_STRING: String = DEFAULT_BUFFER_SIZE.to_string();
}

fn prepare_app() -> App<'static> {
    App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::new("buffer_size")
                .about("Sets input buffer size")
                .short('b')
                .long("buffer-size")
                .takes_value(true)
                .validator(usize_validator)
                .value_name("BUFFER_SIZE")
                .default_value(&DEFAULT_BUFFER_SIZE_STRING)
                .required(false),
        )
        .subcommand(prepare_extract_subcommand())
        .subcommand(prepare_filter_subcommand())
        .subcommand(prepare_trigger_subcommand())
        .subcommand(prepare_convert_subcommand())
        .subcommand(
            App::new("completion").about("completions generator").arg(
                Arg::new("shell")
                    .short('s')
                    .long("shell")
                    .about("For which shell the completion is supposed to be generated")
                    .possible_values(&["bash", "fish", "elvish", "powershell", "zsh"])
                    .required(true),
            ),
        )
}

fn print_completions<G: Generator>(app: &mut App) {
    generate::<G, _>(app, app.get_name().to_string(), &mut io::stdout());
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = prepare_app();

    let arg_matches = app.clone().get_matches();
    let buffer_size: usize = arg_matches.value_of("buffer_size").unwrap().parse()?;
    match arg_matches.subcommand() {
        Some(("convert", matches)) => process_convert(matches, buffer_size),
        Some(("extract", matches)) => process_extract(matches, buffer_size),
        Some(("filter", matches)) => process_filter(matches, buffer_size),
        Some(("trigger", matches)) => process_trigger(matches, buffer_size),
        Some(("completion", matches)) => match matches.value_of("shell") {
            Some("bash") => {
                print_completions::<Bash>(&mut app);
                Ok(())
            }
            Some("elvish") => {
                print_completions::<Elvish>(&mut app);
                Ok(())
            }
            Some("fish") => {
                print_completions::<Fish>(&mut app);
                Ok(())
            }
            Some("powershell") => {
                print_completions::<PowerShell>(&mut app);
                Ok(())
            }
            Some("zsh") => {
                print_completions::<Zsh>(&mut app);
                Ok(())
            }
            _ => unreachable!(),
        },
        _ => {
            app.write_long_help(&mut io::stdout()).unwrap();
            println!();
            process::exit(1);
        }
    }
}
