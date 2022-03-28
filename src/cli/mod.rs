pub mod base_opts;

pub use clap::{AppSettings, Parser, Subcommand};
use clap::{ArgEnum, Args};

use self::base_opts::BaseOpts;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
#[clap(setting(AppSettings::SubcommandRequiredElseHelp))]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Host a game
    Host(HostOptions),

    /// Join a game
    Join(JoinOptions),

    /// Solo practice session
    Practice(PracticeOptions),
}

#[derive(Args, Clone, Debug)]
pub struct HostOptions {
    /// Your username
    #[clap(short, long)]
    pub username: String,

    /// Path to the file to race on
    #[clap(short, long)]
    pub file: String,
}

#[derive(Args, Clone, Debug)]
pub struct JoinOptions {
    /// Your username
    #[clap(short, long)]
    pub username: String,

    /// The host to join
    pub host: String,
}

#[derive(Args, Clone, Debug)]
pub struct PracticeOptions {
    /// Your username
    #[clap(short, long)]
    pub username: Option<String>,

    /// Pick a text file to practice on
    #[clap(short, long)]
    pub file: String,

    /// Race against your past self using an input record from a previous session with this file
    #[clap(short, long)]
    pub ghost: bool,

    /// Update the input record for this file
    #[clap(arg_enum, short, long)]
    pub save: Option<Save>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum Save {
    Best,
    Override,
}

pub fn parse() -> Cli {
    Cli::parse()
}
