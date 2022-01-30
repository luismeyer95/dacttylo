use std::error::Error;

pub type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub enum Action {
    Ok,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coord(pub usize, pub usize);
