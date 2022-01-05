// #![allow(dead_code, unused)]

use dacttylo::{
    editor_state::{Cursor, EditorState},
    editor_view::{EditorRenderer, EditorViewState},
};

#[allow(unused_imports)]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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
    Ok(typebox_app()?)
}

fn typebox_app() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(50);
    let res = run_app(&mut terminal, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    tick_rate: Duration,
) -> Result<(), Box<dyn Error>> {
    let mut last_tick = Instant::now();

    let arg = std::env::args().nth(1).ok_or("No file provided")?;
    let text_content = std::fs::read_to_string(&arg)?;

    let mut editor = EditorState::new().content(&text_content);
    let mut editor_view = EditorViewState::new();

    loop {
        let renderer = EditorRenderer::new().content(editor.get_lines());
        editor_view.focus(editor.get_cursor());

        terminal.draw(|f| {
            f.render_stateful_widget(renderer, f.size(), &mut editor_view);
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
                        if let Some(_) = editor.offset(-1) {
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
