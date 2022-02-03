#![allow(dead_code, unused, clippy::new_without_default)]

use crossterm::cursor::{EnableBlinking, Show};
use dacttylo::{
    editor_state::{Cursor, EditorState},
    editor_view::{EditorRenderer, EditorViewState},
    highlighting::{Highlighter, NoOpHighlighter, SyntectHighlighter},
    utils::{log, types::AsyncResult},
};

#[allow(unused_imports)]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

fn main() -> Result<(), Box<dyn Error>> {
    typebox_app()
}

fn typebox_app() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(5000);
    let res = run_app(&mut terminal, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    tick_rate: Duration,
) -> AsyncResult<()> {
    let mut last_tick = Instant::now();

    let filepath = std::env::args().nth(1);
    let text_content = match &filepath {
        Some(filepath) => std::fs::read_to_string(&filepath)?,
        None => "".into(),
    };

    let mut editor = EditorState::new().content(&text_content);
    let mut editor_view = EditorViewState::new();

    let mut hl_builder = SyntectHighlighter::new()
        .theme("Solarized (dark)")
        .file(filepath)?;

    loop {
        let lines = &editor.get_lines();
        // let hl_lines = hl_builder.clone().build()?.highlight(lines);

        let renderer = EditorRenderer::content(lines);

        editor_view.focus(editor.get_cursor());
        let cursor = editor_view
            .last_render
            .as_ref()
            .and_then(|meta| meta.cursor);

        terminal.draw(|f| {
            f.render_stateful_widget(renderer, f.size(), &mut editor_view);
            if let Some(cursor) = cursor {
                f.set_cursor(cursor.1, cursor.0);
            }
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Enter => {
                        // editor.insert_ln();
                        // editor.move_cursor(Cursor::Down);
                        editor.insert_ch('\n');
                        editor.offset(1);
                    }
                    KeyCode::Tab => {
                        editor.insert_ch('\t');
                        editor.offset(1);
                    }
                    KeyCode::Char(c) => {
                        editor.insert_ch(c);
                        editor.offset(1);
                    }
                    KeyCode::Backspace => {
                        if editor.offset(-1).is_some() {
                            editor.delete_ch();
                        }
                    }
                    KeyCode::Up => editor.move_cursor(Cursor::Up),
                    KeyCode::Down => editor.move_cursor(Cursor::Down),
                    KeyCode::Left => editor.move_cursor(Cursor::Left),
                    KeyCode::Right => editor.move_cursor(Cursor::Right),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // do sth with tick event
            // typebox_state.on_tick();
            last_tick = Instant::now();
        }
    }
}
