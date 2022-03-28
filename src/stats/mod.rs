use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct GameStats {
    pub wpm_series: Vec<(f64, f64)>,
    pub average_wpm: f64,
    pub top_wpm: f64,
    pub precision: f64,
    pub mistake_count: usize,
}

impl fmt::Display for GameStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let wpm = self.wpm_series.last().map_or(0.0, |(_, wpm)| *wpm);

        write!(
            f,
            "Current WPM: {:.2}\nAverage WPM: {:.2}\nTop WPM: {:.2}\nPrecision: {:.2}\nMistakes: {}\n",
            wpm, self.average_wpm, self.top_wpm, self.precision, self.mistake_count
        )
    }
}
