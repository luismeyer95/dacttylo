use std::error::Error;

pub type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub enum Action {
    Ok,
    Quit,
}
