#![allow(dead_code, unused)]
mod highlight;
mod network;
// mod typeview;
mod editor_state;
mod editor_view;
mod highlighter;
mod line_processor;
mod line_stylizer;
mod text_coord;
mod text_view;
mod utils;

use clap::ArgMatches;
use clap::{AppSettings, Arg, Parser};
use editor_state::{Cursor, EditorState};
use editor_view::EditorRenderer;
use network::message;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, queue,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;
use std::{
    borrow::Cow,
    error::Error,
    io,
    path::Prefix,
    time::{Duration, Instant},
};
use tui::text::{self, Text};
use tui::widgets::BorderType;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::editor_view::EditorViewState;
use crate::text_view::Anchor;
use crate::text_view::TextView;

fn is_valid_file(val: &str) -> Result<(), String> {
    if Path::new(val).exists() {
        Ok(())
    } else {
        Err(format!("file `{}` does not exist.", val))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse_opts();
    typebox_app()?;

    Ok(())
}

//////////////////////////////////////////////////////////////////////

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
    let mut index: usize = 0;

    let arg = std::env::args().nth(1).ok_or("No file provided")?;
    let text_content = std::fs::read_to_string(&arg)?;

    let mut editor = EditorState::new().content(&text_content);
    let mut editor_view = EditorViewState::new();

    loop {
        // terminal.draw(|f| ui(f, index).unwrap())?;
        let mut renderer = EditorRenderer::new().content(editor.get_lines());
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
