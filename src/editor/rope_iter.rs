use ropey::{iter::Chunks, str_utils::byte_to_char_idx, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

/// An implementation of a graphemes iterator, for iterating over
/// the graphemes of a RopeSlice.
pub struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemes<'a> {
    pub fn new<'b>(slice: &RopeSlice<'b>) -> RopeGraphemes<'b> {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemes {
            text: *slice,
            chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) =
                        self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            let a_char = self.text.byte_to_char(a);
            let b_char = self.text.byte_to_char(b);

            Some(self.text.slice(a_char..b_char))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((&self.cur_chunk[a2..b2]).into())
        }
    }
}

/// Finds the previous grapheme boundary before the given char position.
pub fn prev_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) =
        slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the previous grapheme cluster boundary.
    loop {
        match gc.prev_boundary(chunk, chunk_byte_idx) {
            Ok(None) => return 0,
            Ok(Some(n)) => {
                let tmp = byte_to_char_idx(chunk, n - chunk_byte_idx);
                return chunk_char_idx + tmp;
            }
            Err(GraphemeIncomplete::PrevChunk) => {
                let (a, b, c, _) = slice.chunk_at_byte(chunk_byte_idx - 1);
                chunk = a;
                chunk_byte_idx = b;
                chunk_char_idx = c;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx_chunk, n - ctx_chunk.len());
            }
            _ => unreachable!(),
        }
    }
}

/// Finds the next grapheme boundary after the given char position.
pub fn next_grapheme_boundary(slice: &RopeSlice, char_idx: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) =
        slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the next grapheme cluster boundary.
    loop {
        match gc.next_boundary(chunk, chunk_byte_idx) {
            Ok(None) => return slice.len_chars(),
            Ok(Some(n)) => {
                let tmp = byte_to_char_idx(chunk, n - chunk_byte_idx);
                return chunk_char_idx + tmp;
            }
            Err(GraphemeIncomplete::NextChunk) => {
                chunk_byte_idx += chunk.len();
                let (a, _, c, _) = slice.chunk_at_byte(chunk_byte_idx);
                chunk = a;
                chunk_char_idx = c;
            }
            Err(GraphemeIncomplete::PreContext(n)) => {
                let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                gc.provide_context(ctx_chunk, n - ctx_chunk.len());
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
#[rustfmt::skip] // Because of the crazy long graphemes
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn iter_huge_graphemes() {
        let r = Rope::from_str("Hẽ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃llõ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃ wõ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃rld!");
        let mut grph = RopeGraphemes::new(&r.slice(..));

        assert_eq!(grph.next().unwrap(), "H");
        assert_eq!(grph.next().unwrap(), "ẽ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃");
        assert_eq!(grph.next().unwrap(), "l");
        assert_eq!(grph.next().unwrap(), "l");
        assert_eq!(grph.next().unwrap(), "õ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃");
        assert_eq!(grph.next().unwrap(), " ");
        assert_eq!(grph.next().unwrap(), "w");
        assert_eq!(grph.next().unwrap(), "õ̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃̃");
        assert_eq!(grph.next().unwrap(), "r");
        assert_eq!(grph.next().unwrap(), "l");
        assert_eq!(grph.next().unwrap(), "d");
        assert_eq!(grph.next().unwrap(), "!");
        assert_eq!(grph.next(), None);
    }


    #[test]
    fn iter_regional_symbols() {
        let r = Rope::from_str("🇬🇧🇯🇵🇺🇸🇫🇷🇷🇺🇨🇳🇩🇪🇪🇸🇬🇧🇯🇵🇺🇸🇫🇷🇷🇺🇨🇳🇩🇪🇪🇸🇬🇧🇯🇵🇺🇸🇫🇷🇷🇺🇨🇳🇩🇪🇪🇸");
        let mut grph = RopeGraphemes::new(&r.slice(..));

        assert_eq!(grph.next().unwrap(), "🇬🇧");
        assert_eq!(grph.next().unwrap(), "🇯🇵");
        assert_eq!(grph.next().unwrap(), "🇺🇸");
        assert_eq!(grph.next().unwrap(), "🇫🇷");
        assert_eq!(grph.next().unwrap(), "🇷🇺");
        assert_eq!(grph.next().unwrap(), "🇨🇳");
        assert_eq!(grph.next().unwrap(), "🇩🇪");
        assert_eq!(grph.next().unwrap(), "🇪🇸");
        assert_eq!(grph.next().unwrap(), "🇬🇧");
        assert_eq!(grph.next().unwrap(), "🇯🇵");
        assert_eq!(grph.next().unwrap(), "🇺🇸");
        assert_eq!(grph.next().unwrap(), "🇫🇷");
        assert_eq!(grph.next().unwrap(), "🇷🇺");
        assert_eq!(grph.next().unwrap(), "🇨🇳");
        assert_eq!(grph.next().unwrap(), "🇩🇪");
        assert_eq!(grph.next().unwrap(), "🇪🇸");
        assert_eq!(grph.next().unwrap(), "🇬🇧");
        assert_eq!(grph.next().unwrap(), "🇯🇵");
        assert_eq!(grph.next().unwrap(), "🇺🇸");
        assert_eq!(grph.next().unwrap(), "🇫🇷");
        assert_eq!(grph.next().unwrap(), "🇷🇺");
        assert_eq!(grph.next().unwrap(), "🇨🇳");
        assert_eq!(grph.next().unwrap(), "🇩🇪");
        assert_eq!(grph.next().unwrap(), "🇪🇸");
        assert_eq!(grph.next(), None);
    }
}
