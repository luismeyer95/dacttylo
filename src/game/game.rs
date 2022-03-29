use std::time::Duration;

use tokio::sync::mpsc::Sender;

use crate::{
    aggregate,
    app::state::{PlayerPool, PlayerState},
    cli::base_opts::BaseOpts,
    events::{app_event, AppEvent, EventAggregator},
    stats::GameStats,
    utils::types::AsyncResult,
};

pub struct Game<'t, O> {
    pub main: PlayerState<'t>,
    pub opponents: PlayerPool<'t>,
    pub stats: GameStats,

    pub client: Sender<AppEvent>,
    pub events: EventAggregator<AppEvent>,
    pub opts: O,

    pub theme: String,
}

impl<'t, O> Game<'t, O>
where
    O: BaseOpts,
{
    pub fn new(
        text: &'t str,
        opponents: &[&str],
        opts: O,
        theme: &str,
    ) -> AsyncResult<Game<'t, O>> {
        let (client, events) = Self::configure_event_stream();

        let username = opts.get_username().unwrap_or("you");

        let main = PlayerState::new(username.to_owned(), text);
        let opponents = PlayerPool::new(text).with_players(opponents);
        let stats = GameStats::default();

        Ok(Game {
            main,
            opponents,
            stats,
            client,
            events,
            opts,
            theme: theme.to_owned(),
        })
    }

    fn configure_event_stream() -> (Sender<AppEvent>, EventAggregator<AppEvent>)
    {
        let (client, stream) = app_event::stream();
        let task_client = client.clone();
        tokio::spawn(async move {
            loop {
                if task_client.send(AppEvent::WpmTick).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });

        let term_io_stream = crossterm::event::EventStream::new();
        (client, aggregate!([stream, term_io_stream] as AppEvent))
    }
}
