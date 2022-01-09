use super::AppEvent;

impl From<Result<crossterm::event::Event, std::io::Error>> for AppEvent {
    fn from(term_event: Result<crossterm::event::Event, std::io::Error>) -> Self {
        AppEvent::Term(term_event)
    }
}
