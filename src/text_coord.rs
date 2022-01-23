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

impl From<(usize, usize)> for TextCoord {
    fn from(coord: (usize, usize)) -> Self {
        Self::new(coord.0, coord.1)
    }
}
