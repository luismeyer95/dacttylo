use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub wpm: f64,
    pub average_wpm: f64,
    pub top_wpm: f64,

    pub precision: f64,
    pub mistake_count: usize,
}

impl fmt::Display for SessionStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Current WPM: {}\nAverage WPM: {}\nTop WPM: {}\nPrecision: {}\nMistakes: {}\n",
            self.wpm, self.average_wpm, self.top_wpm, self.precision, self.mistake_count
        )
    }
}
