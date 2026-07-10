# Neuro Unauthenticated Provenance Boundary Detector

A static analysis tool that detects a specific class of vulnerabilities in the Neuro compiler pipeline: **unauthenticated provenance boundaries** where safety-relevant metadata is produced but never verified by consumers.

## What It Does

The detector identifies boolean fields in Protocol Buffer schemas that:

1. Have **semantic suffixes** indicating safety/security relevance (`_passed`, `_verified`, `_checked`, `_valid`, `_ok`, etc.)
2. Are **written** by producer code (e.g., `analyzer/src/*.rs`)
3. Are **never read** by consumer code (e.g., `backend/*.cpp`)

This pattern indicates a **decorative attestation** — metadata that asserts provenance but doesn't prove it.

## Detection Heuristic

The tool uses a **semantic suffix heuristic** to identify safety-relevant boolean fields:

```rust
const SEMANTIC_SUFFIXES: &[&str] = &[
    "_passed",
    "_verified",
    "_checked",
    "_valid",
    "_ok",
    "_safe",
    "_secure",
    "_auth",
    "_trusted",
    "_complete",
];
```

Fields like `borrow_check_passed` and `type_check_passed` match this heuristic, while fields like `is_async` or `is_mutable` do not.

## Usage

### Against Neuro Compiler (Default)

```bash
cargo run --release
```

This automatically detects the Neuro workspace structure and analyzes:
- Proto schema: `shared_ast/ast.proto`
- Producer: `analyzer/src/*.rs`
- Consumer: `backend/*.cpp`, `backend/*.h`

### Against Custom Paths

```bash
cargo run --release -- <proto_path> <producer_dir> <consumer_dir>
```

### Example Output

```
=== Unauthenticated Provenance Boundary Detector ===

Found 2 field(s) written but never read:

[1] VerifiedProgram.borrow_check_passed (proto line 23)
  Write-sites (producer):
    /path/to/analyzer/src/lib.rs:18 - borrow_check_passed: true,
  Read-sites (consumer): NONE

[2] VerifiedProgram.type_check_passed (proto line 24)
  Write-sites (producer):
    /path/to/analyzer/src/lib.rs:19 - type_check_passed: true,
  Read-sites (consumer): NONE

---
Severity: HIGH (design-level)
Remediation: Consumer must verify semantic bool fields before proceeding.
```

## Tests

### Unit Tests

```bash
cargo test --lib
```

### Integration Tests

```bash
cargo test --test integration_tests
```

The integration tests include three synthetic test cases:

1. **Verified field IS checked** — semantic bool that's read by consumer → zero findings
2. **Non-semantic bool not flagged** — `is_async` field → not flagged (suffix heuristic works)
3. **Neuro-like unchecked semantic bools** — both `borrow_check_passed` and `type_check_passed` flagged

## How It Works

1. **Parse proto schema** — extract boolean fields with semantic suffixes
2. **Search producer code** — find write-sites (assignments to these fields)
3. **Search consumer code** — find read-sites (accessor calls like `.field_name()`)
4. **Compare** — fields with writes but no reads are flagged as findings

## Limitations

This tool is **intentionally scoped** to a specific vulnerability class. It does NOT:

- Detect cryptographic bypasses (e.g., go-tuf threshold=0)
- Analyze runtime behavior (e.g., Docker DCT env var checks)
- Audit CI/CD pipelines (e.g., npm audit signatures)
- Generalize to non-protobuf IDLs (FlatBuffers, MessagePack)

For the full design rationale, see [docs/DESIGN-detector-tool.md](../docs/DESIGN-detector-tool.md) §9 (Out of Scope).

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Detector Tool                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────┐  │
│  │ Proto Parser │    │ Source Search│    │ Reporter │  │
│  │ (semantic    │    │ (regex on    │    │ (plain   │  │
│  │  bools)      │    │  field names)│    │  text)   │  │
│  └──────┬───────┘    └──────┬───────┘    └────┬─────┘  │
│         │                   │                  │        │
│         ▼                   ▼                  ▼        │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Analysis Engine                     │   │
│  │  - Rule 1: Unread Semantic Booleans              │   │
│  │  - Rule 3: Missing Consumer Validation           │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Success Criteria

The tool must:

1. ✅ **Detect the Neuro vulnerability** — flag `borrow_check_passed` and `type_check_passed`
2. ✅ **Not hardcode field names** — derive findings from suffix heuristic + read/write search
3. ✅ **Avoid false positives** — not flag `is_async`, `is_mutable`, or other non-semantic fields
4. ✅ **Provide file:line citations** — exact locations for both write-sites and read-sites

## References

- [Finding Document](../docs/FINDING-unauthenticated-provenance-boundary.md)
- [Design Document](../docs/DESIGN-detector-tool.md)
- [Boundary Test Suite](../tests/boundary_tests.rs)

## License

Part of the Neuro Compiler research artifact. See LICENSE for details.
