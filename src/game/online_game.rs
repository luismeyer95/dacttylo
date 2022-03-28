use super::game::Game;
use crate::session::session_handle::SessionHandle;

pub struct OnlineGame<'t, O> {
    pub session: SessionHandle,
    pub game: Game<'t, O>,
}
