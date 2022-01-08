// #![allow(unused)]

use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use dacttylo::events::ticker::TickerClient;
use dacttylo::events::{ticker, TickEvent};
use dacttylo::session;
use libp2p::{identity, PeerId};
use std::error::Error;
use std::io::Stdout;
use tokio::fs;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tui::backend::CrosstermBackend;
use tui::Terminal;

use dacttylo::events::event_aggregator::EventAggregator;
use dacttylo::{
    self, aggregate,
    events::AppEvent,
    network::{self},
    session::SessionClient,
};
use tokio_stream::StreamExt;

type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AsyncResult<()> {
    if let Err(e) = setup_term().await {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn setup_term() -> AsyncResult<()> {
    let mut term = enter_tui_mode(std::io::stdout())?;
    configure_app(&mut term).await?;
    leave_tui_mode(term)?;

    Ok(())
}

async fn configure_app(term: &mut Terminal<CrosstermBackend<Stdout>>) -> AsyncResult<()> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    println!("Local peer id: {:?}", peer_id);

    let (p2p_client, p2p_stream, task) = network::new(id_keys.clone()).await?;
    // Running the P2P task in the background
    tokio::spawn(task.run());

    let (session_client, session_stream) = session::new(p2p_client, p2p_stream);
    let (ticker_client, ticker_stream) = ticker::new();
    let term_io_stream = crossterm::event::EventStream::new();

    let global_stream = aggregate!([ticker_stream, term_io_stream, session_stream] as AppEvent);

    run_app(term, global_stream, session_client, ticker_client).await?;

    Ok(())
}

async fn run_app(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut global_stream: EventAggregator<AppEvent>,
    session_client: SessionClient,
    ticker_client: TickerClient,
) -> AsyncResult<()> {
    while let Some(event) = global_stream.next().await {
        dacttylo::utils::log(&format!("{:?}", event)).await;

        match event {
            AppEvent::Tick => {}
            AppEvent::Session(_session_event) => {}
            AppEvent::Term(e) => {
                handle_term_event(e.unwrap(), &ticker_client).await;
            }
        }
    }

    Ok(())
}

async fn handle_term_event(term_event: Event, ticker_client: &TickerClient) {
    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        match code {
            KeyCode::Char(c) => {}
            KeyCode::Enter => {
                ticker_client.send(TickEvent).await.unwrap();
            }
            _ => {}
        };
    }
}

fn enter_tui_mode<T>(mut writer: T) -> AsyncResult<Terminal<CrosstermBackend<T>>>
where
    T: std::io::Write,
{
    enable_raw_mode()?;

    execute!(writer, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(writer);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn leave_tui_mode<T>(mut terminal: Terminal<CrosstermBackend<T>>) -> AsyncResult<()>
where
    T: std::io::Write,
{
    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    terminal.show_cursor()?;

    Ok(())
}
