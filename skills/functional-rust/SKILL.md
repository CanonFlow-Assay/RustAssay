---
name: functional-rust
description: Build Rust changes as a pure, immutable domain core with explicit Result and enum outcomes, then place I/O in named adapters.
---

# Functional Rust

Keep domain functions deterministic: take values, return values, and model expected failure with `Result` or an enum. Use `struct` and `enum` data with derived equality for domain values. Pass effect capabilities into shell adapters; do not read files, call the network, print, spawn processes, or terminate control flow from domain modules.

At an untrusted boundary, decode into a dedicated input type, validate once, and then construct trusted domain data. Avoid `unwrap`, `expect`, `panic!`, and placeholder macros on paths that can process ordinary input. Tests must exercise both a successful outcome and an expected error value.
