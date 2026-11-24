# RustAssay

RustAssay checks functional Rust cores with a deterministic `syn` AST scanner. It pairs the scanner with an order domain that models validation as a `Result`, alternatives as an `enum`, and total calculation as a fold. The CLI reads files and resolves the optional Cargo dependency audit.

The repository pins Rust `1.98.1` and uses the Rust 2024 edition.

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- check examples/clean/src/domain --json
cargo run -- check examples/violations/src/domain
```

`check` returns a deterministic JSON receipt with `--json`. The core checks cover fallible methods, panic macros, named effects, ambient time and environment access, and direct indexing. [Contract](docs/CONTRACT.md) · [Functional Rust guidance](skills/functional-rust/SKILL.md)

## Audit runtime dependencies

Pass the core package's manifest to inspect its resolved runtime dependency tree alongside its Rust source.

```sh
rust-assay check crates/core/src --check-deps crates/core/Cargo.toml --json
```

The audit requires a current `Cargo.lock` and locally cached sources. It follows direct and transitive runtime dependencies across Cargo target platforms and checks their library sources for `std::fs` and `std::net` paths. The receipt lists the resolved package IDs, selected features, and observed `#![no_std]` declarations. [Dependency audit contract](docs/CONTRACT.md#runtime-dependency-audit)

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

Empty, linked, unreadable, unparseable, and unsupported source scopes return Inconclusive. Incomplete JSON retains `missingEvidence`, and known findings take precedence as Fail. [Scope completeness](docs/CONTRACT.md#scope-completeness)

## Requirements lifecycle and local archive consumer

The [R6 lifecycle](playground/lifecycle/README.md) freezes the prior12-case
baseline, retains real Unicode-boundary acceptance failures, executes the repaired
22-case candidate and an independent locally packed verifier consumer. Run
`python3 playground/lifecycle/run.py --output ../brainstorm/astra/evidence/native-lifecycle-a-fresh-run`
under the documented exact SDK environment. Its assessment remains separate
from checker authority and human release approval.

Licensed under Apache-2.0.

## Estate evidence

Select the same Rust pin for the engine and the playground, even if the
host default has advanced.

```sh
cargo +1.98.1 test --locked --offline
RUSTUP_TOOLCHAIN=1.98.1 python3 playground/run.py --output artifacts/pinned-expenseguard
```

Keep each mutation disposition and the toolchain identity from `run.json`.
