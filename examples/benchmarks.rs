//
// Copyright (c) 2025 Nathan Fiedler
//
use segmented_array::SegmentedArray;
use std::time::Instant;

//
// This example intends to show that a segmented array will grow in less time
// than a vector, however in practice that may not be the case.
//

fn create_segarray(size: u64) {
    let start = Instant::now();
    let mut coll: SegmentedArray<u64> = SegmentedArray::new();
    for value in 0..size {
        coll.push(value);
    }
    let duration = start.elapsed();
    println!("segarray: {:?}", duration);
}

fn create_vector(size: u64) {
    let start = Instant::now();
    let mut coll: Vec<u64> = Vec::new();
    for value in 0..size {
        coll.push(value);
    }
    let duration = start.elapsed();
    println!("vector: {:?}", duration);
}

fn main() {
    create_segarray(1_000_000_000);
    create_vector(1_000_000_000);
}
