//
// Copyright (c) 2025 Nathan Fiedler
//
use segarray::SegmentedArray;
use std::time::Instant;

//
// This example intends to show that a segmented array will grow in less time
// than a vector, however in practice they are about the same.
//

fn create_segarray(size: usize) -> SegmentedArray<String> {
    let mut coll: SegmentedArray<String> = SegmentedArray::new();
    for _ in 0..size {
        let value = ulid::Ulid::new().to_string();
        coll.push(value);
    }
    coll
}

fn create_vector(size: usize) -> Vec<String> {
    let mut coll: Vec<String> = Vec::new();
    for _ in 0..size {
        let value = ulid::Ulid::new().to_string();
        coll.push(value);
    }
    coll
}

fn main() {
    let start = Instant::now();
    let coll = create_vector(10_000_000);
    let duration = start.elapsed();
    assert_eq!(coll.len(), 10_000_000);
    println!("vector: {:?}", duration);

    let start = Instant::now();
    let coll = create_segarray(10_000_000);
    let duration = start.elapsed();
    assert_eq!(coll.len(), 10_000_000);
    println!("segarray: {:?}", duration);
}
