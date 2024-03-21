# Intrusive Circular Linked List in Rust

## Minimal Supported Rust Version (MSRV)

v1.77.0 since it requires `offset_of!` macro which is stablized in v1.77.0.

## Introduction

The `cdlist` crate implements a non-thread-safe, intrusive, doubly-linked list in Rust. Its primary characteristic is the inclusion of link pointers within the data structures themselves, rather than in separate node wrappers. This approach enhances memory and performance efficiency but requires careful handling of ownership and safety, which this crate has taken care of.

## Characteristics

- **Intrusive Design**: Nodes contain links to their neighbors, reducing overhead.
- **Non-Thread-Safe**: Optimized for single-threaded environments, avoiding the complexity and overhead of synchronization.
- **Self-Ownership**: Nodes own their data and their position within the list. Dropping a data automatically delists it.
- **Memory Safety**: Utilizes pinning to maintain the integrity of self-references within nodes, ensuring safe usage of the data structure.

## Example Usage

```rust
use cdlist::LinkNode;

fn main() {
    let mut node1 = LinkNode::new(1);
    let mut node2 = LinkNode::new(2);

    node1.add(&mut node2); // Adds node2 after node1

    node1.for_each(|&data| println!("{}", data)); // Prints: 1 2
}
```

## Implementation Insights

- **Pinning**: Nodes are pinned (`Pin<Box<Inner<T>>>`) to prevent invalidation of references due to memory movement, crucial for the safety of self-referential structures.

## Next Steps

To really make this crate useful, it needs to allow multi-threading, which can be enabled behind a feature flag. Though, this would involves a lot of work to ensure racing-conditions are handled correctly.

This crate is mainly a learning exercise.
