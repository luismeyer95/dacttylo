pub use clap::{AppSettings, Parser, Subcommand};

#[derive(Parser)]
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
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
