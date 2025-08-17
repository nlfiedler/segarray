//
// Copyright (c) 2025 Nathan Fiedler
//
use segment_array::SegmentArray;
use std::time::Instant;

//
// This example intends to show that a segment array will grow in less time than
// a vector, however in practice that may not be the case.
//

fn create_segarray(size: u64) {
    let start = Instant::now();
    let mut coll: SegmentArray<u64> = SegmentArray::new();
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
    println!("creating SegmentArray...");
    create_segarray(100_000_000);
    println!("creating Vec...");
    create_vector(100_000_000);
}
