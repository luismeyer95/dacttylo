// use crate::{
//     aggregate,
//     app::{state::PlayerPool, widget::DacttyloWidget, InputResult, Progress},
//     events::{app_event, AppEvent, EventAggregator},
//     ghost::Ghost,
//     highlighting::{Highlighter, SyntectHighlighter},
//     input::record::{InputRecorder, RecordManager},
//     utils::types::{Action, AsyncResult},
// };
// use crossterm::event::{Event, KeyCode, KeyEvent};
// use std::io::Stdout;
// use tokio::sync::mpsc::Sender;
// use tokio_stream::StreamExt;
// use tui::{backend::CrosstermBackend, Terminal};

// pub struct PracticeSession<'term, 'file> {
//     term: &'term mut Terminal<CrosstermBackend<Stdout>>,

//     file_name: &'file str,
//     file_contents: &'file str,

//     event_stream: EventAggregator<AppEvent>,
//     ticker: Sender<AppEvent>,

//     highlighter: SyntectHighlighter,
//     recorder: InputRecorder,
//     ghost: Ghost,
// }

// impl<'term, 'file> PracticeSession<'term, 'file> {
//     pub fn new(
//         term: &'term mut Terminal<CrosstermBackend<Stdout>>,
//         file_name: &'file str,
//         file_contents: &'file str,
//     ) -> AsyncResult<Self> {
//         // setup event stream
//         let (ticker, ticker_stream) = app_event::stream();
//         let term_io_stream = crossterm::event::EventStream::new();
//         let event_stream =
//             aggregate!([ticker_stream, term_io_stream] as AppEvent);

//         // init player state
//         let mut game_state =
//             PlayerPool::new("self", &file_contents).with_players(&["ghost"]);
//         let mut ghost = {
//             let input_record = RecordManager::mount_dir("records")?
//                 .load_from_contents(&file_contents)?;
//             Ghost::new(input_record, ticker.clone())
//         };

//         // init the input recorder to create a ghost record after this session
//         let recorder = InputRecorder::new();

//         // style the file contents once, use the result for every frame
//         let lines: Vec<_> = game_state.text().split_inclusive('\n').collect();
//         let highlighter = SyntectHighlighter::new()
//             .file(Some(&file_name))?
//             .theme("base16-mocha.dark")
//             .build()?;
//         let styled_lines = highlighter.highlight(lines.as_ref());

//         Ok(Self {
//             term,
//             file_name,
//             file_contents,
//             ghost,
//             ticker,
//             event_stream,
//             recorder,
//             highlighter,
//         })
//     }

//     pub async fn run(&mut self) -> AsyncResult<()> {
//         self.ticker.send(AppEvent::Tick).await?;
//         self.ghost.start().await?;

//         while let Some(event) = self.event_stream.next().await {
//             match event {
//                 AppEvent::Tick => {}
//                 AppEvent::Term(e) => {
//                     if let Action::Quit =
//                         self.handle_term_event(e?, &mut game_state).await
//                     {
//                         return Ok(());
//                     }
//                 }
//                 AppEvent::GhostInput(c) => {
//                     self.handle_ghost_input(c);
//                 }
//                 _ => {}
//             }

//             self.term.draw(|f| {
//                 f.render_widget(
//                     DacttyloWidget::new(&game_state)
//                         .highlighted_content(styled_lines.clone()),
//                     f.size(),
//                 );
//             })?;
//         }

//         Ok(())
//     }

//     async fn handle_term_event(
//         &mut self,
//         term_event: crossterm::event::Event,
//         game_state: &mut PlayerPool<'_>,
//     ) -> Action {
//         if let Event::Key(event) = term_event {
//             let KeyEvent { code, .. } = event;
//             let c = match code {
//                 KeyCode::Esc => return Action::Quit,
//                 KeyCode::Char(c) => Some(c),
//                 KeyCode::Enter => Some('\n'),
//                 KeyCode::Tab => Some('\t'),
//                 _ => None,
//             };

//             if let Some(c) = c {
//                 self.recorder.push(c);
//                 let input_result = game_state.process_input("self", c).unwrap();

//                 if let InputResult::Correct(Progress::Finished) = input_result {
//                     // let manager = RecordManager::mount_dir("records").unwrap();
//                     // manager.save(game_state.text(), recorder.record()).unwrap();
//                     return Action::Quit;
//                 }
//             }
//         }

//         Action::Ok
//     }

//     fn handle_ghost_input(&self, c: char) {
//         self.game_state.process_input("ghost", c).unwrap();
//     }
// }
