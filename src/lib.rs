//! # rle-decode-helper
//!
//! **THE** fastest way to implement any kind of decoding for **R**un **L**ength **E**ncoded data in Rust.
//!
//! Writing a fast decoder that is also safe can be quite challenging, so this crate is here to save you the
//! hassle of maintaining and testing your own implementation.
//!
//! # Usage
//!
//! ```rust
//! let mut decode_buffer = vec![0, 0, 1, 1, 0, 2, 3];
//! let lookbehind_length = 4;
//! let output_length = 10;
//! rle_decode_helper::rle_decode(&mut decode_buffer, lookbehind_length, output_length);
//! assert_eq!(decode_buffer, [0, 0, 1, 1, 0, 2, 3, 1, 0, 2, 3, 1, 0, 2, 3, 1, 0]);
//! ```

use std::{
    ptr,
    ops,
    cmp,
};

/// Fast decoding of run length encoded data
///
/// Takes the last `lookbehind_length` items of the buffer and repeatedly appends them until
/// `fill_length` items have been copied.
///
/// # Panics
/// * `lookbehind_length` is 0
/// * `lookbehind_length` >= `buffer.len()`
/// * `fill_length + buffer.len()` would overflow
#[inline(always)]
pub fn rle_decode<T>(
    buffer: &mut Vec<T>,
    mut lookbehind_length: usize,
    mut fill_length: usize,
) where T: Copy {
    if lookbehind_length == 0 {zero_repeat_fail()};

    let copy_fragment_start = buffer.len()
        .checked_sub(lookbehind_length)
        .expect("attempt to repeat fragment larger than buffer size");

    // Reserve space for *all* copies
    buffer.reserve(fill_length);

    while fill_length > 0 {
        let fill_size = cmp::min(lookbehind_length, fill_length);
        append_from_within(
            buffer,
            copy_fragment_start..(copy_fragment_start + fill_size)
        );
        fill_length -= fill_size;
        lookbehind_length *= 2;
    }
}

/// Copy of `vec::append_from_within()` proposed for inclusion in stdlib,
/// see https://github.com/rust-lang/rfcs/pull/2714
/// Heavily based on the implementation of `slice::copy_within()`,
/// so we're pretty sure the implementation is sound
#[inline(always)]
fn append_from_within<T, R: ops::RangeBounds<usize>>(seif: &mut Vec<T>, src: R) where T: Copy, {
    let src_start = match src.start_bound() {
        ops::Bound::Included(&n) => n,
        ops::Bound::Excluded(&n) => n
            .checked_add(1)
            .unwrap_or_else(|| vec_index_overflow_fail()),
        ops::Bound::Unbounded => 0,
    };
    let src_end = match src.end_bound() {
        ops::Bound::Included(&n) => n
            .checked_add(1)
            .unwrap_or_else(|| vec_index_overflow_fail()),
        ops::Bound::Excluded(&n) => n,
        ops::Bound::Unbounded => seif.len(),
    };
    assert!(src_start <= src_end, "src end is before src start");
    assert!(src_end <= seif.len(), "src is out of bounds");
    let count = src_end - src_start;
    seif.reserve(count);
    let vec_len = seif.len();
    unsafe {
        // This is safe because reserve() above succeeded,
        // so `seif.len() + count` did not overflow usize
        ptr::copy_nonoverlapping(
            seif.get_unchecked(src_start),
            seif.get_unchecked_mut(vec_len),
            count,
        );
        seif.set_len(vec_len + count);
    }
}

// actually doesn't give any perf advantages, but we're keeping it
// so we don't diverge from the proposed stdlib impl
#[inline(never)]
#[cold]
fn vec_index_overflow_fail() -> ! {
    panic!("attempted to index vec up to maximum usize");
}

// separating this into a function has measurable perf difference
#[inline(never)]
#[cold]
fn zero_repeat_fail() -> ! {
    panic!("attempt to repeat fragment of size 0");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut buf = vec![1, 2, 3, 4, 5];
        rle_decode(&mut buf, 3, 10);
        assert_eq!(buf, &[1, 2, 3, 4, 5, 3, 4, 5, 3, 4, 5, 3, 4, 5, 3]);
    }
    
    #[test]
    fn test_zero_repeat() {
        let mut buf = vec![1, 2, 3, 4, 5];
        rle_decode(&mut buf, 3, 0);
        assert_eq!(buf, &[1, 2, 3, 4, 5]);
    }
    
    #[test]
    #[should_panic]
    fn test_zero_fragment() {
        let mut buf = vec![1, 2, 3, 4, 5];
        rle_decode(&mut buf, 0, 10);
    }
    
    #[test]
    #[should_panic]
    fn test_zero_fragment_and_repeat() {
        let mut buf = vec![1, 2, 3, 4, 5];
        rle_decode(&mut buf, 0, 0);
    }
    
    #[test]
    #[should_panic]
    fn test_overflow_fragment() {
        let mut buf = vec![1, 2, 3, 4, 5];
        rle_decode(&mut buf, 10, 10);
    }
    
    #[test]
    #[should_panic]
    fn test_overflow_buf_size() {
        let mut buf = vec![1, 2, 3, 4, 5];
        rle_decode(&mut buf, 4, usize::max_value());
    }
}
