//
// Copyright (c) 2025 Nathan Fiedler
//
use segment_array::SegmentArray;
use std::time::Instant;

//
// This example was intended to show that the segment array will grow in less
// time than a vector, however in practice that is not the case.
//

fn benchmark_segarray(size: usize) {
    let start = Instant::now();
    let mut coll: SegmentArray<usize> = SegmentArray::new();
    for value in 0..size {
        coll.push(value);
    }
    let duration = start.elapsed();
    println!("segarray create: {:?}", duration);

    // test sequenced access for entire collection
    let start = Instant::now();
    for (index, value) in coll.iter().enumerate() {
        assert_eq!(*value, index);
    }
    let duration = start.elapsed();
    println!("segarray ordered: {:?}", duration);

    let unused = coll.capacity() - coll.len();
    println!("unused capacity: {unused}");

    // test popping all elements from the array
    let start = Instant::now();
    while !coll.is_empty() {
        coll.pop();
    }
    let duration = start.elapsed();
    println!("segarray pop-all: {:?}", duration);
    println!("segarray capacity: {}", coll.capacity());
}

fn benchmark_vector(size: usize) {
    let start = Instant::now();
    let mut coll: Vec<usize> = Vec::new();
    for value in 0..size {
        coll.push(value);
    }
    let duration = start.elapsed();
    println!("vector create: {:?}", duration);

    // test sequenced access for entire collection
    let start = Instant::now();
    for (index, value) in coll.iter().enumerate() {
        assert_eq!(*value, index);
    }
    let duration = start.elapsed();
    println!("vector ordered: {:?}", duration);

    let unused = coll.capacity() - coll.len();
    println!("unused capacity: {unused}");

    // test popping all elements from the vector
    let start = Instant::now();
    while !coll.is_empty() {
        coll.pop();
    }
    let duration = start.elapsed();
    println!("vector pop-all: {:?}", duration);
    println!("vector capacity: {}", coll.capacity());
}

fn main() {
    println!("creating SegmentArray...");
    benchmark_segarray(100_000_000);
    println!("creating Vec...");
    benchmark_vector(100_000_000);
}
