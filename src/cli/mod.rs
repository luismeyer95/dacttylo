use clap::ArgEnum;
pub use clap::{AppSettings, Parser, Subcommand};

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
    Host {
        /// Your username
        #[clap(short, long)]
        user: String,

        /// Path to the file to race on
        #[clap(short, long)]
        file: String,
    },

    /// Join a game
    Join {
        /// The host to join
        host: String,

        /// Your username
        #[clap(short, long)]
        user: String,
    },

    /// Solo practice session
    Practice {
        /// The file to practice on
        #[clap(short, long)]
        file: String,

        /// Replay record inputs for this session
        #[clap(short, long)]
        ghost: bool,

        /// Trigger record state changes after the session
        #[clap(arg_enum, short, long)]
        save: Option<Save>,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum Save {
    Best,
    Override,
}

pub fn parse() -> Cli {
    Cli::parse()
}
