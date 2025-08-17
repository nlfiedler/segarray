# Segment Array

This Rust [crate](https://crates.io/crates/segment-array) contains an implementation of a segment array (also known as a segmented list), as described in this [blog post](https://danielchasehooper.com/posts/segment_array/):

> A data structure with constant time indexing, stable pointers, and works well
> with arena allocators. ... The idea is straight forward: the structure
> contains a fixed sized array of pointers to segments. Each segment is twice
> the size of its predecessor. New segments are allocated as needed. ... Unlike
> standard arrays, pointers to a segment array’s items are always valid because
> items are never moved. Leaving items in place also means it never leaves
> "holes" of abandoned memory in arena allocators. The layout also allows us to
> access any index in constant time.

The functionality, memory layout, and performance of this implementation should be very similar to that of the C implementation.

The overhead of the bit-shifts and logarithm operations required for every push operation seems to outweigh the amortized O(1) of the basic geometrically growing `Vec` array. The main benefit of a segment array is that it works well with arena memory allocators.

This data structure is meant to hold an unknown, though likely large, number of elements, otherwise `Vec` would be more appropriate. An empty array will have a hefty size of around 224 bytes.

For a different but similar data structure, see the [nlfiedler/extarray](https://github.com/nlfiedler/extarray) repository for an implementation of Space-Efficient Extensible Arrays in Rust.

## Examples

A simple example copied from the unit tests.

```rust
let inputs = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];
let mut arr: SegmentArray<String> = SegmentArray::new();
for item in inputs {
    arr.push(item.to_owned());
}
for (idx, elem) in arr.iter().enumerate() {
    assert_eq!(inputs[idx], elem);
}
```

## Supported Rust Versions

The Rust edition is set to `2024` and hence version `1.85.0` is the minimum supported version.

## Troubleshooting

### Memory Leaks

Finding memory leaks with [Address Sanitizer](https://clang.llvm.org/docs/AddressSanitizer.html) is fairly [easy](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html) and seems to work best on Linux. The shell script below gives a quick demonstration of running one of the examples with ASAN analysis enabled.

```shell
#!/bin/sh
env RUSTDOCFLAGS=-Zsanitizer=address RUSTFLAGS=-Zsanitizer=address \
    cargo run -Zbuild-std --target x86_64-unknown-linux-gnu --release --example leak_test
```

## Other Implementations

* [rmeno12/segarray](https://github.com/rmeno12/segarray)
    + Similar to the Zig implementation

## Academic Research

Publications related to the _dynamic array problem_ in order of publication:

* \[1\]: [Resizable Arrays in Optimal Time and Space (1999)](https://www.semanticscholar.org/paper/Resizable-Arrays-in-Optimal-Time-and-Space-Brodnik-Carlsson/7843ee3731560aa81514be409a9ffc42749af289)
    - Section 4 discusses the block sizing and segment/offset computation cost, similar to Segment Arrays.
    - *Data Blocks* are slots and *Super Blocks* are segments.
* \[2\]: [Experiences with the Design and Implementation of Space-Efficient Deques (2001)](https://www.semanticscholar.org/paper/Experiences-with-the-Design-and-Implementation-of-Katajainen-Mortensen/2346307bf5cc3b322ed38e6582cfb854723ebec5)
* \[3\]: [Fast Dynamic Arrays (2017)](https://www.semanticscholar.org/paper/Fast-Dynamic-Arrays-Bille-Christiansen/4f01f5322ef6564d253039a3859ea20f858ac9ef)
* \[4\]: [Immediate-Access Indexing Using Space-Efficient Extensible Arrays (2022)](https://www.semanticscholar.org/paper/Immediate-Access-Indexing-Using-Space-Efficient-Moffat/31e7dd2ee63efa92009035f4f04d9569ed3024c6)
    - Segment Arrays are similar to the **SPACE-EFFICIENT EXTENSIBLE ARRAYS** described in this paper.
    - Similar to **Singly Resizable Arrays** in [1] but doubles the number of slots and segments at the same time.
