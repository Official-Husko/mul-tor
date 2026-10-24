//! Splitting a file into pieces for chunked protocols.

/// `(offset, length)` of each chunk when splitting `total` bytes into pieces of
/// at most `chunk_size`. Yields nothing for `total == 0`.
pub fn chunk_ranges(total: u64, chunk_size: u64) -> impl Iterator<Item = (u64, u64)> {
    assert!(chunk_size > 0, "chunk size must be positive");
    (0..total.div_ceil(chunk_size)).map(move |index| {
        let offset = index * chunk_size;
        (offset, chunk_size.min(total - offset))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_ranges_cover_the_file_exactly() {
        assert_eq!(
            chunk_ranges(10, 4).collect::<Vec<_>>(),
            [(0, 4), (4, 4), (8, 2)]
        );
        assert_eq!(chunk_ranges(8, 4).collect::<Vec<_>>(), [(0, 4), (4, 4)]);
        assert_eq!(chunk_ranges(3, 100).collect::<Vec<_>>(), [(0, 3)]);
        assert_eq!(chunk_ranges(0, 4).count(), 0);
    }
}
