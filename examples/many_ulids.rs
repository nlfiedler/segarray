//
// Copyright (c) 2025 Nathan Fiedler
//
use segment_array::SegmentArray;

//
// Basically useless except that it can be tested with a memory analyzer to
// determine if the segment array is leaking memory. By storing `String` instead
// of numbers, this is slightly more interesting in terms of memory management.
//
fn main() {
    let mut array: SegmentArray<String> = SegmentArray::new();
    // add enough values to allocate a bunch of segments
    for _ in 0..13_000 {
        let value = ulid::Ulid::new().to_string();
        array.push(value);
    }
    // use an into iterator the to visit elements from each of the segments
    for (index, value) in array.into_iter().skip(32).enumerate() {
        if index == 32 {
            println!("32: {value}");
        } else if index == 128 {
            println!("128: {value}");
        } else if index == 336 {
            println!("336: {value}");
        } else if index == 768 {
            println!("768: {value}");
        } else if index == 1024 {
            println!("1024: {value}");
        } else if index == 1600 {
            println!("1600: {value}");
        } else if index == 3084 {
            println!("3084: {value}");
        } else if index == 6666 {
            println!("6666: {value}");
        } else if index == 10000 {
            println!("10000: {value}");
            break;
        }
    }
    // now the Drop implementation for the IntoIter will be invoked and the
    // memory analyzer can catch even more issues
}
