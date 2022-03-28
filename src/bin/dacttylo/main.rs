use dacttylo::cli::Commands;
use dacttylo::utils::types::AsyncResult;

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
        Commands::Practice(practice_opts) => {
            practice::run_practice_session(practice_opts).await?;
        }
        Commands::Host(host_opts) => {
            host::run_host_session(host_opts).await?;
        }
        // Commands::Join { user, host } => {}
        _ => panic!("Command not supported yet"),
    };

    Ok(())
}
