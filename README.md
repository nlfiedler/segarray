# Segmented Array

This crate contains an implmentation of a segmented array, as described in this [blog post](https://danielchasehooper.com/posts/segment_array/):

> A data structure with constant time indexing, stable pointers, and works well
> with arena allocators. ... The idea is straight forward: the structure
> contains a fixed sized array of pointers to segments. Each segment is twice
> the size of its predecessor. New segments are allocated as needed. ... Unlike
> standard arrays, pointers to a segment array’s items are always valid because
> items are never moved. Leaving items in place also means it never leaves
> "holes" of abandoned memory in arena allocators. The layout also allows us to
> access any index in constant time.

In terms of this Rust implementation, rather than stable "pointers", the references returned from `SegmentedArray::get()` will be stable. The behavior, memory layout, and performance of this implementation should be identical to that of the C implementation.

This data structure is meant to hold an unknown, though likely large, number of elements, otherwise `Vec` would be more appropriate. An empty array will have a hefty size of around 224 bytes.

## Requirements

* [Rust](https://www.rust-lang.org) stable (2024 edition)

## Building and Testing

```shell
$ cargo clean
$ cargo build
$ cargo test
```

## Example Usage

Examples can be found in the `examples` directory of the source repository.

```shell
$ cargo run --example many_ulids
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/examples/many_ulids`
value: 01K2AZGB3S7CYBQNRMXHKFPMRG
value: 01K2AZGB46SXSGD2VE77EV88A7
value: 01K2AZGB495KQ2K58GM02MFAZ0
value: 01K2AZGB45D385S5FQB3KM3D4J
value: 01K2AZGB4584FDW5AC837FZMZX
value: 01K2AZGB3RRGV9Y7AHZFDM0T98
value: 01K2AZGB3T6156T5CQB7VX5JR9
value: 01K2AZGB44DE1BGZGPY29FAC8H
value: 01K2AZGB45DHV2QA8EGSMKWHH4
value: 01K2AZGB42REB9DZCAJPS0G0SA
```
