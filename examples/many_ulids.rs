//
// Copyright (c) 2025 Nathan Fiedler
//
use rand::prelude::*;
use segarray::SegmentedArray;

//
// Basically useless except that it can be tested with a memory analyzer to
// determine if the segmented array is leaking memory. By storing `String`
// instead of numbers, this is slightly more interesting in terms of memory
// management.
//
fn main() {
    let mut array: SegmentedArray<String> = SegmentedArray::new();
    // add a lot of values
    for _ in 0..10_000 {
        let value = ulid::Ulid::new().to_string();
        array.push(value);
    }
    // randomly pick 10 entries and print them
    let mut rng = rand::rng();
    for _ in 0..10 {
        let index = rng.random_range(0..10_000);
        let value = array.get(index).unwrap();
        println!("value: {value}");
    }
}
