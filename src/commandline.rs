use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg, SubCommand};

pub fn parse_command() -> App<'static, 'static> {
    return App::new("campfire")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("base-directory")
                .short("b")
                .long("base-directory")
                .help("Directory to build site from")
                .takes_value(true)
                .default_value("."),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Configuration file")
                .takes_value(true)
                .default_value(".campfire/campfire.yaml"),
        )
        .subcommand(SubCommand::with_name("build").about("Builds the site"));
}
