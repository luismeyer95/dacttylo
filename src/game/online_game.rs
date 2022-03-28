use super::game::Game;
use crate::session::session_handle::SessionHandle;

pub struct OnlineGame<'t, O> {
    pub session: SessionHandle,
    pub game: Game<'t, O>,
}

impl<'t, O> OnlineGame<'t, O> {
    pub fn new(session: SessionHandle, game: Game<'t, O>) -> Self {
        OnlineGame { session, game }
    }
}
