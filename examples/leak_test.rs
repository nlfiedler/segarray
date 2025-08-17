//
// Copyright (c) 2025 Nathan Fiedler
//
use segment_array::SegmentArray;

//
// Basically useless except that it can be tested with a memory analyzer to
// determine if the segment array is leaking memory. By storing `String` instead
// of numbers, this is more interesting in terms of memory management since the
// array must drop all of the values, either when the collection is dropped,
// when an IntoIterator is used and eventually dropped.
//
fn main() {
    let mut array: SegmentArray<String> = SegmentArray::new();
    // add enough values to allocate a bunch of segments
    for _ in 0..512 {
        let value = ulid::Ulid::new().to_string();
        array.push(value);
    }
    // explicitly drop the collection to test for leaks
    drop(array);

    // add enough values to allocate a bunch of segments
    let mut array: SegmentArray<String> = SegmentArray::new();
    for _ in 0..4096 {
        let value = ulid::Ulid::new().to_string();
        array.push(value);
    }

    // skip enough elements to pass over a few segments
    for (index, value) in array.into_iter().skip(200).enumerate() {
        if index == 200 {
            println!("200: {value}");
            // exit the iterator early intentionally
            break;
        }
    }
    // IntoIter will be dropped, exposing possible leaks
}
