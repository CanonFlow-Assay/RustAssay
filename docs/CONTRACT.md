# RustAssay v0.1 contract

RustAssay is a deterministic lexical checker for selected functional-core hazards. It scans `.rs` files recursively beneath the supplied directory and returns a stable, sorted report. Exit `0` means no selected pattern was observed; exit `1` means a selected pattern was found; exit `2` means the target could not be read; exit `64` means invalid arguments.

Rules are intentionally narrow:

- `RSA-F001`: `.unwrap()` and `.expect(`;
- `RSA-F002`: panic and placeholder macros;
- `RSA-F003`: named filesystem, network, process, and console effects.

Lexical evidence does not understand comments, macro expansion, type information, aliases, or runtime behavior. A clean result does not prove purity, correctness, security, or idiomaticity. It only records that these selected fragments were absent from the scanned UTF-8 source.

## Scope-completeness increment (29 October 2026)

A missing or empty supported-source scope returns Inconclusive (exit 2). The shell refuses symbolic-link roots/ancestors and entries, nonregular source files, traversal/enumeration failures, malformed UTF-8 and unsupported size/depth/count limits. Limits are 10,000 source files, 64 directory levels, 4 MiB per file and 64 MiB total. A known lexical finding remains Fail (exit 1) even when another input is incomplete. JSON retains findings and adds `missingEvidence` when needed. Existing nonempty complete Pass/Fail receipts retain their schema and rule meanings.

The pure receipt renderer also refuses to grant Pass for zero observed files. The shell assumes a trusted local, quiescent filesystem; it is not a hostile-host sandbox or semantic compiler. Existing lexical false positives, aliases/macros/comments and runtime limitations remain.
