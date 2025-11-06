use itertools::Itertools;

use crate::bit_positions::BitposIteratorExt;

mod bit_positions;

#[repr(C)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryRange {
    pub gpa: u64,
    pub length: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryRangeTable {
    data: Vec<MemoryRange>,
}

impl MemoryRangeTable {
    pub fn from_bitmap(bitmap: Vec<u64>, start_addr: u64, page_size: u64) -> Self {
        let mut table = MemoryRangeTable::default();
        let mut entry: Option<MemoryRange> = None;
        for (i, block) in bitmap.iter().enumerate() {
            for j in 0..64 {
                let is_page_dirty = ((block >> j) & 1u64) != 0u64;
                let page_offset = ((i * 64) + j) as u64 * page_size;
                if is_page_dirty {
                    if let Some(entry) = &mut entry {
                        entry.length += page_size;
                    } else {
                        entry = Some(MemoryRange {
                            gpa: start_addr + page_offset,
                            length: page_size,
                        });
                    }
                } else if let Some(entry) = entry.take() {
                    table.push(entry);
                }
            }
        }
        if let Some(entry) = entry.take() {
            table.push(entry);
        }

        table
    }

    pub fn dirty_range_iter(
        bitmap: impl IntoIterator<Item = u64>,
        start_addr: u64,
        page_size: u64,
    ) -> impl Iterator<Item = MemoryRange> {
        bitmap
            .into_iter()
            .bit_positions()
            // Turn them into single-element ranges for coalesce.
            .map(|b| b..(b + 1))
            // Merge adjacent ranges.
            .coalesce(|prev, curr| {
                if prev.end == curr.start {
                    Ok(prev.start..curr.end)
                } else {
                    Err((prev, curr))
                }
            })
            .map(move |r| MemoryRange {
                gpa: start_addr + r.start * page_size,
                length: (r.end - r.start) * page_size,
            })
    }
    pub fn from_bitmap_iter(
        bitmap_iter: impl IntoIterator<Item = u64>,
        start_addr: u64,
        page_size: u64,
    ) -> Self {
        MemoryRangeTable {
            data: Self::dirty_range_iter(bitmap_iter, start_addr, page_size).collect(),
        }
    }

    pub fn push(&mut self, range: MemoryRange) {
        self.data.push(range)
    }
}

pub fn bitmap_to_memory_table(bitmap1: &[u64], bitmap2: &[u64]) -> MemoryRangeTable {
    assert_eq!(bitmap1.len(), bitmap2.len());

    let dirty_bitmap: Vec<u64> = bitmap1
        .iter()
        .zip(bitmap2.iter())
        .map(|(x, y)| x | y)
        .collect();

    MemoryRangeTable::from_bitmap(dirty_bitmap, 0, 4096)
}

pub fn bitmap_to_memory_table2(bitmap1: &[u64], bitmap2: &[u64]) -> MemoryRangeTable {
    assert_eq!(bitmap1.len(), bitmap2.len());

    let dirty_bitmap_iter = bitmap1.iter().zip(bitmap2.iter()).map(|(x, y)| x | y);

    MemoryRangeTable::from_bitmap_iter(dirty_bitmap_iter, 0, 4096)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{collection::vec, prop_assert_eq, proptest};

    proptest! {
        #[test]
        fn bitmap_functions_are_identical(
            bitmap1 in vec(0u64..u64::MAX, 0..100),
            bitmap2 in vec(0u64..u64::MAX, 0..100)
        ) {
            // Ensure both bitmaps have the same length
            let len = bitmap1.len().min(bitmap2.len());
            let bitmap1 = &bitmap1[..len];
            let bitmap2 = &bitmap2[..len];

            let result1 = bitmap_to_memory_table(bitmap1, bitmap2);
            let result2 = bitmap_to_memory_table2(bitmap1, bitmap2);

            prop_assert_eq!(result1, result2);
        }
    }
}
