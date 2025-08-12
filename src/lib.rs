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
use std::iter::{FromIterator, Iterator};
use std::ops::Index;

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
const SMALL_SEGMENTS_CAPACITY: usize = 1 << SMALL_SEGMENTS_TO_SKIP;

// Calculates the number of elements that will fit into the given segment.
#[inline]
fn slots_in_segment(segment: usize) -> usize {
    SMALL_SEGMENTS_CAPACITY << segment
}

// Calculates the overall capacity for all segments up to the given segment.
#[inline]
fn capacity_for_segment_count(segment: usize) -> usize {
    (SMALL_SEGMENTS_CAPACITY << segment) - SMALL_SEGMENTS_CAPACITY
}

const LOG2I_BASE: i32 = 8 * (std::mem::size_of::<usize>() as i32) - 1;

// Integer base-2 logarithm function to compute the segment for a given offset
// within the segmented array, identical to that of the C implementation.
#[inline]
fn log2i(value: usize) -> i32 {
    // #define log2i(X) ((u32) (8*sizeof(unsigned long long) - __builtin_clzll((X)) - 1))
    LOG2I_BASE - value.leading_zeros() as i32
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
            // overflowing the allocator is very unlikely as the item size would
            // have to be very large
            let layout = Layout::array::<T>(segment_len).expect("unexpected overflow");
            unsafe {
                let ptr = alloc(layout).cast::<T>();
                if ptr.is_null() {
                    handle_alloc_error(layout);
                }
                self.segments[self.used_segments] = ptr;
            }
            self.used_segments += 1;
        }

        let segment = log2i((self.count >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
        let slot = (self.count - capacity_for_segment_count(segment)) as isize;
        unsafe {
            let end: *mut T = self.segments[segment].offset(slot);
            std::ptr::write(end, value);
        }
        self.count += 1;
    }

    /// Removes the last element from a vector and returns it, or `None` if it
    /// is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.count > 0 {
            self.count -= 1;
            let segment = log2i((self.count >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
            let slot = (self.count - capacity_for_segment_count(segment)) as isize;
            unsafe { Some((self.segments[segment].offset(slot)).read()) }
        } else {
            None
        }
    }

    /// Return the number of elements in the array.
    ///
    /// # Time complexity
    ///
    /// Constant time.
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// Returns true if the array has a length of 0.
    pub fn is_empty(&self) -> bool {
        self.count == 0
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
            let segment = log2i((index >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
            let slot = (index - capacity_for_segment_count(segment)) as isize;
            unsafe { (self.segments[segment].offset(slot)).as_ref() }
        }
    }

    /// Returns an iterator over the segmented array.
    ///
    /// The iterator yields all items from start to end.
    pub fn iter(&self) -> SegArrayIter<'_, T> {
        SegArrayIter {
            array: self,
            index: 0,
        }
    }

    /// Clears the segmented array, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// segmented array.
    pub fn clear(&mut self) {
        if self.count > 0 {
            if std::mem::needs_drop::<T>() {
                // find the last segment that contains values
                let last_segment = log2i((self.count >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
                let last_slot = self.count - capacity_for_segment_count(last_segment);
                unsafe {
                    std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                        self.segments[last_segment],
                        last_slot,
                    ));
                }
                // now drop the values in all of the preceding segments
                for segment in 0..last_segment {
                    let segment_len = slots_in_segment(segment);
                    unsafe {
                        std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                            self.segments[segment],
                            segment_len,
                        ));
                    }
                }
            }
            self.count = 0;
        }
    }
}

impl<T> Index<usize> for SegmentedArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let Some(item) = self.get(index) else {
            panic!("index out ouf bounds: {}", index);
        };
        item
    }
}

impl<A> FromIterator<A> for SegmentedArray<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut arr: SegmentedArray<A> = SegmentedArray::new();
        for value in iter {
            arr.push(value)
        }
        arr
    }
}

/// Immutable segmented array iterator.
pub struct SegArrayIter<'a, T> {
    array: &'a SegmentedArray<T>,
    index: usize,
}

impl<'a, T> Iterator for SegArrayIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index);
        self.index += 1;
        value
    }
}

/// An iterator that moves out of a segmented array.
pub struct SegArrayIntoIter<T> {
    index: usize,
    count: usize,
    used_segments: usize,
    segments: [*mut T; MAX_SEGMENT_COUNT],
}

impl<T> Iterator for SegArrayIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let segment = log2i((self.index >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
            let slot = (self.index - capacity_for_segment_count(segment)) as isize;
            self.index += 1;
            unsafe { Some((self.segments[segment].offset(slot)).read()) }
        } else {
            None
        }
    }
}

impl<T> Drop for SegArrayIntoIter<T> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            let first_segment = log2i((self.index >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
            let last_segment = log2i((self.count >> SMALL_SEGMENTS_TO_SKIP) + 1) as usize;
            if first_segment == last_segment {
                // special-case, remaining values are in only one segment
                let first_slot = self.index - capacity_for_segment_count(first_segment);
                let last_slot = self.count - capacity_for_segment_count(first_segment);
                if first_slot < last_slot {
                    let len = last_slot - first_slot;
                    unsafe {
                        let first: *mut T =
                            self.segments[first_segment].offset(first_slot as isize);
                        std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(first, len));
                    }
                }
            } else {
                let first_slot = self.index - capacity_for_segment_count(first_segment);
                let segment_len = slots_in_segment(first_segment);
                if segment_len < self.count {
                    unsafe {
                        let first: *mut T =
                            self.segments[first_segment].offset(first_slot as isize);
                        let len = segment_len - first_slot;
                        std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(first, len));
                    }
                }

                // drop the values in the last segment
                let last_slot = self.count - capacity_for_segment_count(last_segment);
                unsafe {
                    std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                        self.segments[last_segment],
                        last_slot,
                    ));
                }

                // now drop the values in all of the other segments
                if last_segment > first_segment {
                    for segment in first_segment + 1..last_segment {
                        let segment_len = slots_in_segment(segment);
                        unsafe {
                            std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                                self.segments[segment],
                                segment_len,
                            ));
                        }
                    }
                }
            }
        }

        // deallocate the segments themselves and clear everything
        for segment in 0..self.used_segments {
            if !self.segments[segment].is_null() {
                let segment_len = slots_in_segment(segment);
                let layout = Layout::array::<T>(segment_len).expect("unexpected overflow");
                unsafe {
                    dealloc(self.segments[segment] as *mut u8, layout);
                }
                self.segments[segment] = std::ptr::null_mut();
            }
        }
        self.index = 0;
        self.count = 0;
        self.used_segments = 0;
    }
}

impl<T> IntoIterator for SegmentedArray<T> {
    type Item = T;
    type IntoIter = SegArrayIntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let me = std::mem::ManuallyDrop::new(self);
        SegArrayIntoIter {
            index: 0,
            count: me.count,
            used_segments: me.used_segments,
            segments: me.segments,
        }
    }
}

impl<T> Drop for SegmentedArray<T> {
    fn drop(&mut self) {
        // perform the drop_in_place() for all of the values
        self.clear();
        // deallocate the segments themselves and clear everything
        for segment in 0..self.used_segments {
            if !self.segments[segment].is_null() {
                let segment_len = slots_in_segment(segment);
                let layout = Layout::array::<T>(segment_len).expect("unexpected overflow");
                unsafe {
                    dealloc(self.segments[segment] as *mut u8, layout);
                }
                self.segments[segment] = std::ptr::null_mut();
            }
        }
        self.used_segments = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slots_in_segment() {
        // values are simply capacity_for_segment_count() plus 64 but there
        // should be a test for this function regardless of its simplicity
        let expected_values = [
            64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288,
            1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 67108864, 134217728, 268435456,
            536870912, 1073741824, 2147483648, 4294967296,
        ];
        assert_eq!(expected_values.len(), MAX_SEGMENT_COUNT + 1);
        for segment in 0..=MAX_SEGMENT_COUNT {
            assert_eq!(expected_values[segment], slots_in_segment(segment));
        }
    }

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
        assert!(sut.is_empty());
        sut.push(item);
        assert_eq!(sut.len(), 1);
        assert!(!sut.is_empty());
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
        assert_eq!(sut[3], "four");
    }

    #[test]
    fn test_push_and_pop() {
        let inputs = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        ];
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        assert!(sut.pop().is_none());
        for item in inputs {
            sut.push(item.to_owned());
        }
        assert_eq!(sut.len(), 9);
        for (idx, elem) in sut.iter().enumerate() {
            assert_eq!(inputs[idx], elem);
        }
        let maybe = sut.pop();
        assert!(maybe.is_some());
        let value = maybe.unwrap();
        assert_eq!(value, "nine");
        assert_eq!(sut.len(), 8);
        sut.push(String::from("nine"));
        assert_eq!(sut.len(), 9);
        for (idx, elem) in sut.iter().enumerate() {
            assert_eq!(inputs[idx], elem);
        }
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
        assert_eq!(sut[99], 99);
    }

    #[test]
    fn test_clear_and_reuse_tiny() {
        // clear an array that allocated only one segment
        let inputs = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        ];
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for item in inputs {
            sut.push(item.to_owned());
        }
        assert_eq!(sut.len(), 9);
        sut.clear();
        assert_eq!(sut.len(), 0);
        for item in inputs {
            sut.push(item.to_owned());
        }
        assert_eq!(sut.len(), 9);
        // implicitly drop()
    }

    #[test]
    fn test_clear_and_reuse_ints() {
        let mut sut: SegmentedArray<i32> = SegmentedArray::new();
        for value in 0..512 {
            sut.push(value);
        }
        assert_eq!(sut.len(), 512);
        sut.clear();
        assert_eq!(sut.len(), 0);
        for value in 0..512 {
            sut.push(value);
        }
        for idx in 0..512 {
            let maybe = sut.get(idx);
            assert!(maybe.is_some(), "{idx} is none");
            let actual = maybe.unwrap();
            assert_eq!(idx, *actual as usize);
        }
    }

    #[test]
    fn test_clear_and_reuse_strings() {
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for _ in 0..512 {
            let value = ulid::Ulid::new().to_string();
            sut.push(value);
        }
        assert_eq!(sut.len(), 512);
        sut.clear();
        assert_eq!(sut.len(), 0);
        for _ in 0..512 {
            let value = ulid::Ulid::new().to_string();
            sut.push(value);
        }
        assert_eq!(sut.len(), 512);
        // implicitly drop()
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
        assert_eq!(sut[99_999], 99_999);
    }

    #[test]
    fn test_array_iterator() {
        let inputs = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        ];
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for item in inputs {
            sut.push(item.to_owned());
        }
        for (idx, elem) in sut.iter().enumerate() {
            assert_eq!(inputs[idx], elem);
        }
    }

    #[test]
    fn test_array_intoiterator() {
        // an array that only requires a single segment
        let inputs = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        ];
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for item in inputs {
            sut.push(item.to_owned());
        }
        for (idx, elem) in sut.into_iter().enumerate() {
            assert_eq!(inputs[idx], elem);
        }
        // sut.len(); // error: ownership of sut was moved
    }

    #[test]
    fn test_array_intoiterator_drop_tiny() {
        // an array that only requires a single segment and only some need to be
        // dropped after partially iterating the values
        let inputs = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        ];
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for item in inputs {
            sut.push(item.to_owned());
        }
        for (idx, _) in sut.into_iter().enumerate() {
            if idx > 2 {
                break;
            }
        }
        // implicitly drop()
    }

    #[test]
    fn test_array_intoiterator_drop_large() {
        // by adding 512 values and iterating less than 64 times, there will be
        // values in the first segment and some in the last segment, and two
        // segments inbetween that all need to be dropped
        let mut sut: SegmentedArray<String> = SegmentedArray::new();
        for _ in 0..512 {
            let value = ulid::Ulid::new().to_string();
            sut.push(value);
        }
        for (idx, _) in sut.into_iter().enumerate() {
            if idx >= 30 {
                break;
            }
        }
        // implicitly drop()
    }

    #[test]
    fn test_array_fromiterator() {
        let mut inputs: Vec<i32> = Vec::new();
        for value in 0..10_000 {
            inputs.push(value);
        }
        let sut: SegmentedArray<i32> = inputs.into_iter().collect();
        assert_eq!(sut.len(), 10_000);
        for idx in 0..10_000i32 {
            let maybe = sut.get(idx as usize);
            assert!(maybe.is_some(), "{idx} is none");
            let actual = maybe.unwrap();
            assert_eq!(idx, *actual as i32);
        }
    }

    #[test]
    fn test_add_get_many_instances() {
        // test allocating, filling, and then dropping many instances
        for _ in 0..1_000 {
            let mut sut: SegmentedArray<usize> = SegmentedArray::new();
            for value in 0..10_000 {
                sut.push(value);
            }
            assert_eq!(sut.len(), 10_000);
        }
    }
}
