use crate::session::event::SessionEvent;

#[derive(Debug)]
pub enum AppEvent {
    Tick,
    Term(Result<crossterm::event::Event, std::io::Error>),
    Session(SessionEvent),
    Ghost(char),
}
