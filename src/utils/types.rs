use std::error::Error;

use tui::text::StyledGrapheme;

pub type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub enum Action {
    Ok,
    Quit,
}

pub type StyledLine<'a> = Vec<StyledGrapheme<'a>>;
