# RustAssay v0.1 contract

`rust-assay check <directory> [--json] [--check-deps <Cargo.toml>]` recursively parses `.rs` files in the declared directory and sorts findings by path, line, column, and rule ID. Locations use one-based lines and character columns. The parser keeps comments and literal text out of rule matching and accepts multiline Rust syntax.

| Exit | Meaning |
| --- | --- |
| `0` | Pass. Complete observations under the requested checks produced zero findings. |
| `1` | Fail. At least one admitted rule produced a finding. |
| `2` | Inconclusive. Required observations are missing. |
| `64` | Invalid command arguments. |

## Functional-core rules

| Rule | Matched syntax |
| --- | --- |
| `RSA-F001` | `.unwrap()` and `.expect(...)` method calls. |
| `RSA-F002` | `panic!`, `unreachable!`, `todo!`, and `unimplemented!` invocations. |
| `RSA-F003` | Paths or imports rooted at `std::fs`, `std::net`, or `std::process`, and `println!` and `eprintln!` invocations. |
| `RSA-F004` | Paths or imports rooted at `std::time` or `std::env`, and `env!` and `option_env!` invocations. Pass timestamps and configuration as inputs and use `core::time::Duration` for duration values. |
| `RSA-F005` | Index expressions including slices, range indexing, and writes through indexing. Use `.get()` or `.get_mut()` and handle the missing case. |

RustAssay reads grouped and renamed imports and checks the Rust expression arguments of common assertion, formatting, and `vec!` macros. General macro expansion, name resolution, type resolution, and conditional compilation require separate evidence. Rules operate on the syntax as written, including conditional branches and names that may be shadowed. Purity, correctness, and runtime behavior require separate verification.

Library callers of `rust_assay::inspect` receive a `Result<Vec<Finding>, syn::Error>`. Pass a complete Rust file and retain a parse error as missing evidence when producing a receipt.

## Runtime dependency audit

`--check-deps <Cargo.toml>` selects one Cargo package. Supply the package manifest and a source directory within that package. RustAssay runs `cargo metadata --format-version 1 --frozen --all-features --manifest-path <Cargo.toml>`. Cargo supplies the all-feature resolution across targets. The audit uses the existing lockfile and local cache and runs metadata without compiling the package. [Cargo metadata reference](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)

The audit follows normal dependency edges from the selected package using resolved package IDs. Its runtime scope excludes build, dev, and proc-macro edges. RustAssay visits each runtime dependency's library target and its explicit module files. `RSA-F006` reports visible `std::fs` or `std::net` paths and imports in those sources, including paths in transitive dependencies and conservatively inspected conditional code.

The optional JSON `dependencyAudit` object records the selected manifest and a sorted list of runtime package IDs, library entry points, and resolved features. `declaresNoStd` records the unconditional `#![no_std]` attribute when the library root parses, with `null` for an unavailable root. The filesystem and network checks run throughout the resolved graph, including beneath crates with `#![no_std]`. Explicit `include!` of external Rust, unresolved module files, and conditional module path selection produce missing evidence. The policy covers named Rust references. Assess aliases, macro-generated code, native I/O, and actual platform behavior separately.

## Scope completeness

An empty, missing, or unreadable source scope, an unreadable or unparseable `.rs` file, or an unavailable requested dependency graph produces missing evidence. The source walk also checks symbolic links, nonregular files, and traversal limits. The source scan and dependency library scan each allow 10,000 files, 64 levels of traversal, 4 MiB per file, and 64 MiB total. The Cargo metadata reader allows 32 MiB of stdout and a 30-second deadline. The CLI operates on trusted local projects and filesystem state.

A known finding retains Fail when another part of the scan is incomplete. The `rust-assay/v0.1` JSON schema keeps the existing fields and adds `missingEvidence` when needed. `scannedFiles` counts successfully parsed `.rs` files in the declared source directory. The optional dependency audit has its own observations.
