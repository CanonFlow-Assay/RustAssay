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

## ExpenseGuard playground

The [native ExpenseGuard playground](playground/README.md) adds requirements,
real baseline/expanded acceptance execution, isolated mutation controls, bounded
offline investigation advice and source/log freshness checks. Run:

```sh
python3 playground/run.py --output artifacts/playground/first-run
```

This experiment is separate from checker authority. Exact runtime pin gaps remain
Inconclusive even when the experiment completes; survivors remain visible.

## Scope completeness

Empty, linked, unreadable and unsupported source scopes cannot Pass. Incomplete JSON retains `missingEvidence`; known findings remain Fail. See the [current scope addendum](docs/CONTRACT.md#scope-completeness-increment-29-october-2026). Original USB evaluation metadata remains historical; its current-resolution section records this increment.
