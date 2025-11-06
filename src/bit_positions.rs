/// An iterator that turns a sequence of u64s into a sequence of bit positions that are set.
struct BitposIterator<I> {
    underlying_it: I,

    /// How many u64's we've already consumed.
    word_pos: usize,

    // If we already started working on a u64, it's here. Together with the bit
    // position where we have to continue.
    current_word: Option<(u64, u32)>,
}

impl<I> Iterator for BitposIterator<I>
where
    I: Iterator<Item = u64>,
{
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_word.is_none() {
            self.current_word = self.underlying_it.next().map(|w| (w, 0));
        }

        if let Some((word, word_bit)) = self.current_word {
            let shifted_word = if word_bit < 64 { word >> word_bit } else { 0 };

            if shifted_word == 0 {
                self.current_word = None;
                self.word_pos += 1;
                self.next()
            } else {
                let zeroes = shifted_word.trailing_zeros();

                assert!(zeroes + word_bit <= 64);
                if zeroes + word_bit == 64 {
                    self.current_word = None;
                    self.word_pos += 1;
                    self.next()
                } else {
                    self.current_word = Some((word, zeroes + word_bit + 1));
                    let next_bitpos =
                        u64::try_from(self.word_pos).unwrap() * 64 + u64::from(word_bit + zeroes);

                    Some(next_bitpos)
                }
            }
        } else {
            None
        }
    }
}

pub trait BitposIteratorExt: Iterator<Item = u64> + Sized {
    fn bit_positions(self) -> impl Iterator<Item = u64> {
        BitposIterator {
            underlying_it: self,
            word_pos: 0,
            current_word: None,
        }
    }
}

impl<I: Iterator<Item = u64> + Sized> BitposIteratorExt for I {}

#[cfg(test)]
mod tests {
    use super::*;

    fn bitpos_check(inp: &[u64], out: &[u64]) {
        assert_eq!(inp.iter().copied().bit_positions().collect::<Vec<_>>(), out);
    }

    #[test]
    fn bitpos_iterator_works() {
        bitpos_check(&[], &[]);
        bitpos_check(&[0], &[]);
        bitpos_check(&[1], &[0]);
        bitpos_check(&[5], &[0, 2]);
        bitpos_check(&[3 + 32], &[0, 1, 5]);

        bitpos_check(&[1, 1 + 32], &[0, 64, 69]);
    }
}
