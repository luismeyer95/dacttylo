use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tui::{backend::CrosstermBackend, Terminal};

use super::AppEvent;

// pub enum TermCommand {
//     EnterTUI,
//     LeaveTUI,
// }

// pub struct TermIO {
//     pub stream: crossterm::event::EventStream,
//     pub client: Sender<TermCommand>,
//     rx: Receiver<TermCommand>,
// }

// impl TermIO {
//     pub fn new() -> Self {
//         let (tx, rx) = mpsc::channel::<TermCommand>(256);
//         Self {
//             stream: crossterm::event::EventStream::new(),
//             client: tx,
//             rx,
//         }
//     }

//     pub async fn task(&mut self) {
//         while let Some(command) = self.rx.recv().await {
//             match command {
//                 TermCommand::EnterTUI => {
//                     enable_raw_mode().unwrap();
//                     let mut stdout = std::io::stdout();
//                     execute!(stdout, EnterAlternateScreen).unwrap();
//                     // let backend = CrosstermBackend::new(stdout);
//                     // let mut terminal = Terminal::new(backend).unwrap();
//                 }
//                 TermCommand::LeaveTUI => {
//                     // restore terminal
//                     disable_raw_mode().unwrap();
//                     execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
//                     terminal.show_cursor()?;
//                 }
//             }
//         }
//     }
// }

impl From<Result<crossterm::event::Event, std::io::Error>> for AppEvent {
    fn from(term_event: Result<crossterm::event::Event, std::io::Error>) -> Self {
        AppEvent::Term(term_event)
    }
}
