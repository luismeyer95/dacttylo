use super::processor::{BaseLineProcessor, LineProcessor, SymbolMap};
use tui::{
    style::{Color, Style},
    text::StyledGrapheme,
};

const TAB_SYMBOL: &str = "\u{21e5}";
const NL_SYMBOL: &str = "\u{23ce}";

pub struct LineStylizer;

impl LineProcessor for LineStylizer {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        width: u16,
        default_bg: Color,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let yellow = |symbol| StyledGrapheme {
            style: Style::default().fg(Color::Yellow),
            symbol,
        };

        let processor = BaseLineProcessor {
            symbols: SymbolMap {
                tab: yellow(TAB_SYMBOL),
                nl: yellow(NL_SYMBOL),
            },
        };

        processor.process_line(line, width, default_bg)
    }
}
