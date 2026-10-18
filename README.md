# RustAssay

RustAssay is a small, deterministic checker for a functional Rust core. It pairs a pure lexical scanner with a worked order domain where validation is a `Result`, alternatives are an `enum`, and total calculation is a fold. The shell that reads files lives only in the CLI.

The repository pins [Rust `1.98.1`](https://blog.rust-lang.org/releases/), the current stable release at its creation. It uses the Rust 2024 edition and has no runtime dependencies or telemetry.

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- check examples/clean/src/domain --json
cargo run -- check examples/violations/src/domain
```

`check` returns a deterministic JSON receipt with `--json`. A clean receipt means only that the three documented lexical rules did not match the scanned source; read [the contract](docs/CONTRACT.md) for its evidence boundary. The reusable authoring guidance is in [functional-rust](skills/functional-rust/SKILL.md).

Licensed under Apache-2.0.
