mod highlighter;
mod noop;
mod syntect;

pub use self::{
    highlighter::Highlighter,
    noop::NoOpHighlighter,
    syntect::{SyntectHighlighter, SyntectHighlighterBuilder},
};
