#[cfg(feature = "man")]
use man::prelude::*;
#[cfg(feature = "man")]
include!("src/docs.rs");
#[cfg(feature = "man")]
include!("src/rules.rs");

#[cfg(feature = "man")]
fn root_page() -> man::Manual {
    Manual::new("sson")
        .about("A memory efficient tool to process large JSONs data.")
        .flag(
            Flag::new()
                .short("-h")
                .long("--help")
                .help("Prints help information"),
        )
        .flag(
            Flag::new()
                .short("-V")
                .long("--version")
                .help("Prints version information"),
        )
        .option(
            Opt::new("BUFFER_SIZE")
                .short("-b")
                .long("--buffer-size")
                .help("Sets input buffer size [default: 1048576]"),
        )
        .arg(Arg::new("<strategy>"))
        .arg(Arg::new("[<args>]"))
        .custom(
            Section::new("strategy")
                .paragraph("sson-all(1)")
                .paragraph("sson-convert(1)")
                .paragraph("sson-extract(1)")
                .paragraph("sson-filter(1)")
                .paragraph("sson-trigger(1)"),
        )
}

#[cfg(feature = "man")]
fn all_page() -> man::Manual {
    let handlers_section = make_man_section(
        &handlers::MAP,
        Some(
            &handlers_for_strategy("all")
                .into_iter()
                .collect::<Vec<&str>>(),
        ),
        "handlers",
    );

    Manual::new("sson-all")
        .about(strategies::All.description())
        .flag(Flag::new().long("--help").help("Prints help information"))
        .flag(
            Flag::new()
                .short("-V")
                .long("--version")
                .help("Prints version information"),
        )
        .option(
            Opt::new("HANDLER")
                .short("-h")
                .long("--handler")
                .help("see HANDLERS section"),
        )
        .custom(handlers_section)
}

#[cfg(feature = "man")]
fn convert_page() -> man::Manual {
    let handlers_section = make_man_section(
        &handlers::MAP,
        Some(
            &handlers_for_strategy("convert")
                .into_iter()
                .collect::<Vec<&str>>(),
        ),
        "handlers",
    );
    let matchers_section = make_man_section(&matchers::MAP, None, "matchers");

    Manual::new("sson-convert")
        .about(strategies::Convert.description())
        .flag(Flag::new().long("--help").help("Prints help information"))
        .flag(
            Flag::new()
                .short("-V")
                .long("--version")
                .help("Prints version information"),
        )
        .option(
            Opt::new("MATCHER")
                .short("-m")
                .long("--matcher")
                .help("see MATCHERS section"),
        )
        .option(
            Opt::new("HANDLER")
                .short("-h")
                .long("--handler")
                .help("see HANDLERS section"),
        )
        .custom(matchers_section)
        .custom(handlers_section)
}

#[cfg(feature = "man")]
fn extract_page() -> man::Manual {
    let handlers_section = make_man_section(
        &handlers::MAP,
        Some(
            &handlers_for_strategy("extract")
                .into_iter()
                .collect::<Vec<&str>>(),
        ),
        "handlers",
    );
    let matchers_section = make_man_section(&matchers::MAP, None, "matchers");

    Manual::new("sson-extract")
        .about(strategies::Extract.description())
        .flag(Flag::new().long("--help").help("Prints help information"))
        .flag(
            Flag::new()
                .short("-V")
                .long("--version")
                .help("Prints version information"),
        )
        .option(
            Opt::new("MATCHER")
                .short("-m")
                .long("--matcher")
                .help("see MATCHERS section"),
        )
        .option(
            Opt::new("HANDLER")
                .short("-h")
                .long("--handler")
                .help("see HANDLERS section"),
        )
        .option(
            Opt::new("START")
                .short("-b")
                .long("--before")
                .help("Will be printed to stdout before first match"),
        )
        .option(
            Opt::new("SEPARATOR")
                .short("-S")
                .long("--separator")
                .help("Separator which will be inserted between matched parts"),
        )
        .option(
            Opt::new("END")
                .short("-a")
                .long("--after")
                .help("Will be printed to stdout before first match"),
        )
        .custom(matchers_section)
        .custom(handlers_section)
}

#[cfg(feature = "man")]
fn filter_page() -> man::Manual {
    let handlers_section = make_man_section(
        &handlers::MAP,
        Some(
            &handlers_for_strategy("filter")
                .into_iter()
                .collect::<Vec<&str>>(),
        ),
        "handlers",
    );
    let matchers_section = make_man_section(&matchers::MAP, None, "matchers");

    Manual::new("sson-filter")
        .about(strategies::Filter.description())
        .flag(Flag::new().long("--help").help("Prints help information"))
        .flag(
            Flag::new()
                .short("-V")
                .long("--version")
                .help("Prints version information"),
        )
        .option(
            Opt::new("MATCHER")
                .short("-m")
                .long("--matcher")
                .help("see MATCHERS section"),
        )
        .option(
            Opt::new("HANDLER")
                .short("-h")
                .long("--handler")
                .help("see HANDLERS section"),
        )
        .custom(matchers_section)
        .custom(handlers_section)
}

#[cfg(feature = "man")]
fn trigger_page() -> man::Manual {
    let handlers_section = make_man_section(
        &handlers::MAP,
        Some(
            &handlers_for_strategy("trigger")
                .into_iter()
                .collect::<Vec<&str>>(),
        ),
        "handlers",
    );
    let matchers_section = make_man_section(&matchers::MAP, None, "matchers");

    Manual::new("sson-trigger")
        .about(strategies::Trigger.description())
        .flag(Flag::new().long("--help").help("Prints help information"))
        .flag(
            Flag::new()
                .short("-V")
                .long("--version")
                .help("Prints version information"),
        )
        .option(
            Opt::new("MATCHER")
                .short("-m")
                .long("--matcher")
                .help("see MATCHERS section"),
        )
        .option(
            Opt::new("HANDLER")
                .short("-h")
                .long("--handler")
                .help("see HANDLERS section"),
        )
        .custom(matchers_section)
        .custom(handlers_section)
}

#[cfg(feature = "man")]
fn create_man_page() {
    let out_dir = std::env::var("MANPAGE_DIR").unwrap_or(std::env::var("OUT_DIR").unwrap());
    std::fs::create_dir_all(&out_dir).expect("Can't create manpage dir");

    let base_path = std::path::PathBuf::from(out_dir);

    let mut root_path = base_path.clone();
    root_path.push("sson.1");
    std::fs::write(root_path, root_page().render()).expect("Error writing man page");

    let mut all_path = base_path.clone();
    all_path.push("sson-all.1");
    std::fs::write(all_path, all_page().render()).expect("Error writing man page");

    let mut convert_path = base_path.clone();
    convert_path.push("sson-convert.1");
    std::fs::write(convert_path, convert_page().render()).expect("Error writing man page");

    let mut extract_path = base_path.clone();
    extract_path.push("sson-extract.1");
    std::fs::write(extract_path, extract_page().render()).expect("Error writing man page");

    let mut filter_path = base_path.clone();
    filter_path.push("sson-filter.1");
    std::fs::write(filter_path, filter_page().render()).expect("Error writing man page");

    let mut trigger_path = base_path.clone();
    trigger_path.push("sson-trigger.1");
    std::fs::write(trigger_path, trigger_page().render()).expect("Error writing man page");
}

fn main() {
    #[cfg(feature = "man")]
    create_man_page();
}
