use dacttylo::cli::Commands;
use dacttylo::utils::types::AsyncResult;

mod app;
mod common;
// mod host;
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
            // host::run_host_session(host_opts).await?;
        }
        // Commands::Join { user, host } => {}
        _ => panic!("Command not supported yet"),
    };

    Ok(())
}

// async fn init_multi_session(
//     term: &mut Terminal<CrosstermBackend<Stdout>>,
// ) -> AsyncResult<()> {
//     let mut term = enter_tui_mode(std::io::stdout())?;

//     let id_keys = identity::Keypair::generate_ed25519();
//     let peer_id = PeerId::from(id_keys.public());

//     // println!("Local peer id: {:?}", peer_id);

//     let (p2p_client, p2p_stream, p2p_task) =
//         network::new(id_keys.clone()).await?;
//     // Running the P2P task in the background
//     tokio::spawn(p2p_task.run());

//     let (session_client, session_stream) = session::new(p2p_client, p2p_stream);
//     let (ticker_client, ticker_stream) = ticker::new();
//     let term_io_stream = crossterm::event::EventStream::new();

//     let global_stream =
//         aggregate!([ticker_stream, term_io_stream, session_stream] as AppEvent);

//     run_multi_session(&mut term, global_stream, session_client, ticker_client)
//         .await?;

//     leave_tui_mode(term)?;

//     Ok(())
// }

// async fn run_multi_session(
//     term: &mut Terminal<CrosstermBackend<Stdout>>,
//     mut global_stream: EventAggregator<AppEvent>,
//     session_client: SessionClient,
//     ticker_client: TickerClient,
// ) -> AsyncResult<()> {
//     let cli = dacttylo::cli::parse();
//     let text_contents: String;

//     let mut game_state = match cli.command {
//         Commands::Practice { file } => {
//             text_contents = std::fs::read_to_string(file)?;
//             DacttyloGameState::new("Luis", &text_contents)
//                 .with_players(&["Agathe"])
//         }
//         _ => panic!("Command not supported yet"),
//     };

//     let ticker = ticker_client.clone();
//     tokio::spawn(async move {
//         loop {
//             let rd = rand::thread_rng().gen_range(100..700);
//             tokio::time::sleep(Duration::from_millis(rd)).await;
//             ticker.tick().await.unwrap();
//         }
//     });

//     term.draw(|f| {
//         f.render_widget(DacttyloWidget::new(&game_state), f.size());
//     })?;

//     while let Some(event) = global_stream.next().await {
//         // dacttylo::utils::log(&format!("{:?}", event)).await;

//         match event {
//             AppEvent::Tick => {
//                 game_state.advance_player("Agathe").unwrap();
//             }
//             AppEvent::Session(_session_event) => {}
//             AppEvent::Term(e) => {
//                 if let Action::Quit = handle_term_event(
//                     e.unwrap(),
//                     &ticker_client,
//                     &mut game_state,
//                 )
//                 .await
//                 {
//                     return Ok(());
//                 }
//             }
//         }

//         term.draw(|f| {
//             f.render_widget(DacttyloWidget::new(&game_state), f.size());
//         })?;
//     }

//     Ok(())
// }

// async fn handle_term_event(
//     term_event: Event,
//     ticker_client: &TickerClient,
//     game_state: &mut DacttyloGameState<'_>,
// ) -> Action {
//     if let Event::Key(event) = term_event {
//         let KeyEvent { code, .. } = event;
//         match code {
//             KeyCode::Char(c) => {
//                 game_state.process_input("Luis", c).unwrap();
//             }
//             KeyCode::Enter => {
//                 game_state.process_input("Luis", '\n').unwrap();
//                 // ticker_client.send(TickEvent).await.unwrap();
//             }
//             KeyCode::Tab => {
//                 game_state.process_input("Luis", '\t').unwrap();
//                 // ticker_client.send(TickEvent).await.unwrap();
//             }
//             KeyCode::Esc => return Action::Quit,
//             _ => {}
//         };
//     }

//     Action::Ok
// }
