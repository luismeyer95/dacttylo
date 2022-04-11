#![allow(unused)]

use dacttylo::cli::Commands;
use dacttylo::utils::types::AsyncResult;
use host::run_host_session;
use join::run_join_session;
use practice::run_practice_session;

mod app;
mod common;
mod host;
mod join;
mod practice;
mod protocol;
mod report;

#[tokio::main]
async fn main() -> AsyncResult<()> {
    dacttylo::cli::parse();

    if let Err(e) = init_session().await {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn init_session() -> AsyncResult<()> {
    let cli = dacttylo::cli::parse();

    match cli.command {
        Commands::Practice(opts) => run_practice_session(opts).await?,
        Commands::Host(opts) => run_host_session(opts).await?,
        Commands::Join(opts) => run_join_session(opts).await?,
    };

    Ok(())
}
