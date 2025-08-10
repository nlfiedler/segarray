//
// Copyright (c) 2025 Nathan Fiedler
//

//! An append-only (no insert or remove) growable array as described in the
//! [blog post](https://danielchasehooper.com/posts/segment_array/) by Daniel
//! Hooper.
//!
//! From the blog post:
//!
//! > A data structure with constant time indexing, stable pointers, and works
//! > well with arena allocators. ... The idea is straight forward: the
//! > structure contains a fixed sized array of pointers to segments. Each
//! > segment is twice the size of its predecessor. New segments are allocated
//! > as needed. ... Unlike standard arrays, pointers to a segment array’s items
//! > are always valid because items are never moved. Leaving items in place
//! > also means it never leaves "holes" of abandoned memory in arena
//! > allocators. The layout also allows us to access any index in constant
//! > time.
//!
//! In terms of this Rust implementation, rather than stable "pointers", the
//! references returned from [`SegmentedArray::get()`] will be stable. The
//! behavior, memory layout, and performance of this implementation should be
//! identical to that of the C implementation. To summarize:
//! 
//! * Fixed number of segments (26)
//! * First segment has a capacity of 64
//! * Each segment is double the size of the previous one
//! * The total capacity if 4,294,967,232 items
//!
//! This data structure is meant to hold an unknown, though likely large, number
//! of elements, otherwise `Vec` would be more appropriate. An empty array will
//! have a hefty size of around 224 bytes.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};

//
// An individual segment can never be larger than 9,223,372,036,854,775,807
// bytes due to the mechanics of the Rust memory allocator.
//
// 26 segments with 6 skipped segments can hold 4,294,967,232 items
//
// 9,223,372,036,854,775,807 bytes divided by 4,294,967,232 items yields a
// maximum item size of 2,147,483,680 bytes
//
const MAX_SEGMENT_COUNT: usize = 26;

// Segments of size 1, 2, 4, 8, 16, and 32 are not used at all (that is, the
// smallest (first) segment is 64 elements in size) to avoid the overhead of
// such tiny arrays.
const SMALL_SEGMENTS_TO_SKIP: usize = 6;

// Calculates the number of elements that will fit into the given segment.
#[inline]
fn slots_in_segment(segment: usize) -> usize {
    (1 << SMALL_SEGMENTS_TO_SKIP) << segment
}

// Calculates the overall capacity for all segments up to the given segment.
#[inline]
fn capacity_for_segment_count(segment: usize) -> usize {
    ((1 << SMALL_SEGMENTS_TO_SKIP) << segment) - (1 << SMALL_SEGMENTS_TO_SKIP)
}

// #define _direct_index(sa, segment, slot) (typeof((sa)->payload))(((u8 *)(sa)->internal.segments[segment] + slot * sizeof(*(sa)->payload)))
// fn sa_for(sa) ->
//     for (typeof(*(sa)->payload) it, *it_ptr, *k = (void *)1; k; k=0) \
//         for(u32 segment=0, it_index=0; it_index < sa_count(sa); segment++) \
//             for(u32 slot=0; slot < slots_in_segment(segment) && it_index < sa_count(sa) && (it_ptr=_direct_index((sa), segment, slot), it=*it_ptr, 1) ; slot++, it_index++)

// Integer logarithm function to compute the segment for a given offset within
// the segmented array, identical to that of the C implementation.
#[inline]
fn log2i(value: u32) -> i32 {
    //
    // #define log2i(X) ((u32) (8*sizeof(unsigned long long) - __builtin_clzll((X)) - 1))
    //
    // assume that unsigned long long is equivalent to u64, hence 8 * 4 = 32
    // (and minus 1 yields 31)
    31 - value.leading_zeros() as i32
}

///
/// Append-only growable array that uses a list of progressivly larger segments
/// to avoid the allocate-and-copy that typical growable data structures employ.
///
pub struct SegmentedArray<T> {
    count: usize,
    used_segments: usize,
    segments: [*mut T; MAX_SEGMENT_COUNT],
}

impl<T> SegmentedArray<T> {
    /// Return an empty segmented array with zero capacity.
    ///
    /// Note that pre-allocating capacity has no benefit with this data
    /// structure since append operations are always constant time.
    pub fn new() -> Self {
        Self {
            count: 0,
            used_segments: 0,
            segments: [0 as *mut T; MAX_SEGMENT_COUNT],
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    ///
    /// Panics if a new segment is allocated that would exceed `isize::MAX` _bytes_.
    ///
    /// # Time complexity
    ///
    /// Constant time.
    pub fn push(&mut self, value: T) {
        if self.count >= capacity_for_segment_count(self.used_segments) {
            assert!(
                self.used_segments < MAX_SEGMENT_COUNT,
                "maximum number of segments exceeded"
            );
            let segment_len = slots_in_segment(self.used_segments);
            unsafe {
                // overflowing the allocator is very unlikely as the item size
                // would have to be very large
                let layout = Layout::array::<T>(segment_len).expect("unexpected overflow");
                let ptr = alloc(layout).cast::<T>();
                if ptr.is_null() {
                    handle_alloc_error(layout);
                }
                self.segments[self.used_segments] = ptr;
                self.used_segments += 1;
            }
        }

        let segment = log2i((self.count >> SMALL_SEGMENTS_TO_SKIP) as u32 + 1) as usize;
        let slot = (self.count - capacity_for_segment_count(segment)) as isize;
        // unsafe { *self.segments[segment].offset(slot) = value }
        unsafe {
            let end: *mut T = self.segments[segment].offset(slot);
            std::ptr::write(end, value);
        }
        self.count += 1;
    }

    /// Return the number of elements in the array.
    ///
    /// # Time complexity
    ///
    /// Constant time.
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// Retrieve a reference to the element at the given offset.
    ///
    /// # Time complexity
    ///
    /// Constant time.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.count {
            None
        } else {
            let segment = log2i((index >> SMALL_SEGMENTS_TO_SKIP) as u32 + 1) as usize;
            let slot = (index - capacity_for_segment_count(segment)) as isize;
            unsafe { (self.segments[segment].offset(slot)).as_ref() }
        }
    }
}

impl<T> Drop for SegmentedArray<T> {
    fn drop(&mut self) {
        for segment in 0..self.used_segments {
            let segment_len = slots_in_segment(segment);
            unsafe {
                let layout = Layout::array::<T>(segment_len).expect("unexpected overflow");
                dealloc(self.segments[segment] as *mut u8, layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capacity_for_segment_count() {
        //
        // from https://danielchasehooper.com/posts/segment_array/segment_array.h:
        //
        // 26 segments with 6 skipped segments can hold 4,294,967,232 items, aka
        // capacity_for_segment_count(26)
        //
        let expected_values = [
            0, 64, 192, 448, 960, 1984, 4032, 8128, 16320, 32704, 65472, 131008, 262080, 524224,
            1048512, 2097088, 4194240, 8388544, 16777152, 33554368, 67108800, 134217664, 268435392,
            536870848, 1073741760, 2147483584, 4294967232,
        ];
        assert_eq!(expected_values.len(), MAX_SEGMENT_COUNT + 1);
        for count in 0..=MAX_SEGMENT_COUNT {
            assert_eq!(expected_values[count], capacity_for_segment_count(count));
        }
    }

    #[test]
    fn test_log2i() {
        assert_eq!(log2i(0), -1);
        assert_eq!(log2i(1), 0);
        assert_eq!(log2i(2), 1);
        assert_eq!(log2i(4), 2);
        assert_eq!(log2i(11), 3);
        assert_eq!(log2i(64), 6);
        assert_eq!(log2i(192), 7);
        assert_eq!(log2i(448), 8);
        assert_eq!(log2i(960), 9);
        assert_eq!(log2i(1984), 10);
        assert_eq!(log2i(4032), 11);
        assert_eq!(log2i(8128), 12);
        assert_eq!(log2i(16320), 13);
        assert_eq!(log2i(32704), 14);
        assert_eq!(log2i(65472), 15);
        assert_eq!(log2i(131008), 16);
        assert_eq!(log2i(262080), 17);
        assert_eq!(log2i(524224), 18);
        assert_eq!(log2i(1048512), 19);
        assert_eq!(log2i(2097088), 20);
        assert_eq!(log2i(4194240), 21);
        assert_eq!(log2i(8388544), 22);
        assert_eq!(log2i(16777152), 23);
        assert_eq!(log2i(33554368), 24);
        assert_eq!(log2i(67108800), 25);
        assert_eq!(log2i(134217664), 26);
        assert_eq!(log2i(268435392), 27);
        assert_eq!(log2i(536870848), 28);
        assert_eq!(log2i(1073741760), 29);
        assert_eq!(log2i(2147483584), 30);
        assert_eq!(log2i(4294967232), 31);
    }

    #[test]
    fn test_add_get_one_item() {
        let item = String::from("hello world");
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        assert_eq!(sut.len(), 0);
        sut.push(item);
        assert_eq!(sut.len(), 1);
        let maybe = sut.get(0);
        assert!(maybe.is_some());
        let actual = maybe.unwrap();
        assert_eq!("hello world", actual);
        let missing = sut.get(10);
        assert!(missing.is_none());
    }

    #[test]
    fn test_add_get_several_strings() {
        let inputs = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        ];
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for item in inputs {
            sut.push(item.to_owned());
        }
        assert_eq!(sut.len(), 9);
        for idx in 0..=8 {
            let maybe = sut.get(idx);
            assert!(maybe.is_some(), "{idx} is none");
            let actual = maybe.unwrap();
            assert_eq!(inputs[idx], actual);
        }
        let maybe = sut.get(10);
        assert!(maybe.is_none());
    }

    #[test]
    fn test_add_get_thousands_structs() {
        struct MyData {
            a: u64,
            b: i32,
        }
        let mut sut: SegmentedArray<MyData> = SegmentedArray::new();
        for value in 0..88_888i32 {
            sut.push(MyData {
                a: value as u64,
                b: value,
            });
        }
        assert_eq!(sut.len(), 88_888);
        for idx in 0..88_888i32 {
            let maybe = sut.get(idx as usize);
            assert!(maybe.is_some(), "{idx} is none");
            let actual = maybe.unwrap();
            assert_eq!(idx as u64, actual.a);
            assert_eq!(idx, actual.b);
        }
    }

    #[test]
    fn test_add_get_hundred_ints() {
        let mut sut: SegmentedArray<i32> = SegmentedArray::new();
        for value in 0..100 {
            sut.push(value);
        }
        assert_eq!(sut.len(), 100);
        for idx in 0..100 {
            let maybe = sut.get(idx);
            assert!(maybe.is_some(), "{idx} is none");
            let actual = maybe.unwrap();
            assert_eq!(idx, *actual as usize);
        }
    }

    #[test]
    fn test_add_get_many_ints() {
        let mut sut: SegmentedArray<i32> = SegmentedArray::new();
        for value in 0..1_000_000 {
            sut.push(value);
        }
        assert_eq!(sut.len(), 1_000_000);
        for idx in 0..1_000_000 {
            let maybe = sut.get(idx);
            assert!(maybe.is_some(), "{idx} is none");
            let actual = maybe.unwrap();
            assert_eq!(idx, *actual as usize);
        }
    }

    #[test]
    fn test_add_get_many_instances() {
        // test allocating, filling, and then dropping many instances
        for _ in 0..10_000 {
            let mut sut: SegmentedArray<usize> = SegmentedArray::new();
            for value in 0..10_000 {
                sut.push(value);
            }
            assert_eq!(sut.len(), 10_000);
        }
    }
}
