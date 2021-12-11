#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct TextCoord {
    pub ln: usize,
    pub x: usize,
}

impl TextCoord {
    pub fn new(ln: usize, x: usize) -> Self {
        Self { ln, x }
    }
}
