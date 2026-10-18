# RustAssay v0.1 contract

RustAssay is a deterministic lexical checker for selected functional-core hazards. It scans `.rs` files recursively beneath the supplied directory and returns a stable, sorted report. Exit `0` means no selected pattern was observed; exit `1` means a selected pattern was found; exit `2` means the target could not be read; exit `64` means invalid arguments.

Rules are intentionally narrow:

- `RSA-F001`: `.unwrap()` and `.expect(`;
- `RSA-F002`: panic and placeholder macros;
- `RSA-F003`: named filesystem, network, process, and console effects.

Lexical evidence does not understand comments, macro expansion, type information, aliases, or runtime behavior. A clean result does not prove purity, correctness, security, or idiomaticity. It only records that these selected fragments were absent from the scanned UTF-8 source.
