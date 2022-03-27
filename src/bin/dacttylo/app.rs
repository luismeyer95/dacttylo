use dacttylo::{
    app::state::{PlayerPool, PlayerState},
    events::{AppEvent, EventAggregator},
};
use tokio::sync::mpsc::Sender;

const THEME: &str = "Solarized (dark)";

pub struct Game<'t, O> {
    pub main: PlayerState<'t>,
    pub opponents: PlayerPool<'t>,

    pub client: Sender<AppEvent>,
    pub events: EventAggregator<AppEvent>,
    pub opts: O,

    pub theme: &'static str,
}

impl<'t, O> Game<'t, O> {
    pub fn from(
        main: PlayerState<'t>,
        opponents: PlayerPool<'t>,
        client: Sender<AppEvent>,
        events: EventAggregator<AppEvent>,
        opts: O,
    ) -> Self {
        Game {
            main,
            opponents,
            client,
            events,
            opts,
            theme: THEME,
        }
    }
}
