use super::types::AsyncResult;
use crossterm::{
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use tui::{backend::CrosstermBackend, Terminal};
use unicode_segmentation::UnicodeSegmentation;

pub fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
}

pub fn enter_tui_mode<T>(
    mut writer: T,
) -> AsyncResult<Terminal<CrosstermBackend<T>>>
where
    T: std::io::Write,
{
    enable_raw_mode()?;

    execute!(writer, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(writer);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

pub fn leave_tui_mode<T>(
    mut terminal: Terminal<CrosstermBackend<T>>,
) -> AsyncResult<()>
where
    T: std::io::Write,
{
    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    terminal.show_cursor()?;

    Ok(())
}
