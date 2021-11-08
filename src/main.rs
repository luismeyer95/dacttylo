#![allow(dead_code, unused)]
mod network;
use clap::{load_yaml, ArgMatches};
use clap::{AppSettings, Arg, Parser};
use network::message;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use once_cell::sync::OnceCell;
use std::path::Path;
use std::{
    error::Error,
    io,
    path::Prefix,
    time::{Duration, Instant},
};
use tui::text::{self, Text};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

fn parse_opts() -> &'static ArgMatches {
    static OPTS: OnceCell<ArgMatches> = OnceCell::new();
    OPTS.get_or_init(|| {
        clap::App::new("typebox")
            .arg(
                Arg::new("file")
                    .about("the input file to use")
                    .index(1)
                    .required(true)
                    .validator(is_valid_file),
            )
            .get_matches()
    })
}

fn file_to_string(s: &str) -> Result<String, io::Error> {
    std::fs::read_to_string(Path::new(s))
}

fn prettify_text(s: &str) -> String {
    s.replace('\t', "\u{21e5}   ").replace('\n', "\u{23ce}\n")
}

fn is_valid_file(val: &str) -> Result<(), String> {
    if Path::new(val).exists() {
        Ok(())
    } else {
        Err(format!("file `{}` does not exist.", val))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    parse_opts();
    typebox_app()?;
    Ok(())
}

//////////////////////////////////////////////////////////////////////

fn typebox_app() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen,)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(200);
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
    loop {
        terminal.draw(|f| ui(f).unwrap())?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
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

fn ui<B: Backend>(f: &mut Frame<B>) -> Result<(), Box<dyn Error>> {
    let filename = parse_opts().value_of("file").unwrap();
    let text_content = file_to_string(filename)?;

    let size = f.size();
    let block = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
    f.render_widget(block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(5)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(size);

    let paragraph = Paragraph::new(text_content)
        .style(Style::default())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default()),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .scroll((0, 0));
    f.render_widget(paragraph, chunks[0]);

    Ok(())
}