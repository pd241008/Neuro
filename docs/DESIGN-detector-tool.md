# Design Document: Unauthenticated Provenance Boundary Detector

**Target:** Neuro compiler pipeline (protobuf/IDL-mediated boundaries)
**Scope:** Static analysis of source code to detect provenance boundary vulnerabilities
**NOT in scope:** Universal supply-chain scanner, Docker/npm/K8s patterns, runtime verification

---

## 1. Problem Statement

Phase 1 (empirical audit) found that the "unauthenticated provenance boundary"
vulnerability manifests differently across systems:

| System | Boundary Type | Detection Signal |
|--------|---------------|------------------|
| Neuro | Protobuf field + separate consumer binary | Boolean field in IDL, consumer ignores it |
| Docker | Environment variable opt-in | `DOCKER_CONTENT_TRUST` not enforced |
| npm | Separate CLI command | `npm audit signatures` not in install flow |
| K8s | Missing code path | Webhook not called for subresources |

**Key insight:** A single heuristic cannot generalize across all these patterns.
This detector is scoped to the **protobuf/IDL-mediated pattern** specifically —
the class where:

1. A producer writes safety/security metadata into serialized fields
2. A consumer deserializes but never reads those fields
3. The boundary crosses a process, language, or serialization format

This is the pattern that affects Neuro and is most amenable to static analysis.

---

## 2. Detection Strategy

### 2.1 What We're Looking For

The detector should identify **producer-consumer mismatches** in IDL-mediated
systems:

**Producer side signals:**
- Boolean fields in protobuf/FlatBuffers/MessagePack schemas with semantic names
  (`*_passed`, `*_verified`, `*_checked`, `*_valid`, `*_ok`)
- Code that hardcodes these fields to `true` or `false`
- Code that sets these fields based on validation logic

**Consumer side signals:**
- Deserialization of the IDL message
- Access to message fields *except* the boolean metadata fields
- No conditional logic on the boolean fields
- No error handling for `field == false`

### 2.2 Detection Rules

#### Rule 1: Unread Semantic Boolean Fields

**Pattern:** In an IDL schema (`.proto`, `.fbs`, `.capnp`), boolean fields with
semantic names (`*_passed`, `*_verified`, etc.) that are never read by any
consumer code.

**Implementation:**
1. Parse IDL schema, extract all `bool` fields with semantic suffixes
2. For each field, search consumer code for references (field access, method calls)
3. Flag fields that are written by producers but never read by consumers

**Confidence:** HIGH — direct evidence of decorative attestation

#### Rule 2: Hardcoded True/False in Producer

**Pattern:** Producer code that sets boolean metadata fields to constant values
(`true`, `false`) rather than conditional logic.

**Implementation:**
1. Identify IDL message types used in producer code
2. Find assignment sites for boolean fields
3. Flag assignments where the value is a literal (not a variable or expression)

**Confidence:** MEDIUM — may be intentional defaults that are checked elsewhere

#### Rule 3: Missing Consumer Validation

**Pattern:** Consumer deserializes IDL message but has no conditional branch on
boolean metadata fields.

**Implementation:**
1. Identify IDL message types used in consumer code
2. Map all field accesses in consumer
3. Flag messages where semantic boolean fields exist but are never accessed

**Confidence:** HIGH — direct evidence of unchecked attestation

#### Rule 4: Cross-Language/Cross-Process Boundary

**Pattern:** IDL schema is shared between code in different languages or
different binary targets.

**Implementation:**
1. Parse build files (Cargo.toml, CMakeLists.txt, Makefile, package.json)
2. Identify IDL compilation targets (protoc, flatc, capnp)
3. Check if producer and consumer are in different language directories or
   separate binary targets

**Confidence:** LOW — contextual signal, not direct evidence

---

## 3. Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Detector Tool                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────┐  │
│  │ IDL Parser   │    │ Source Parser │    │ Reporter │  │
│  │ (.proto,     │    │ (Rust, C++,  │    │ (JSON,   │  │
│  │  .fbs, etc.) │    │  Python, etc)│    │  SARIF)  │  │
│  └──────┬───────┘    └──────┬───────┘    └────┬─────┘  │
│         │                   │                  │        │
│         ▼                   ▼                  ▼        │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Analysis Engine                     │   │
│  │  - Rule 1: Unread Semantic Booleans              │   │
│  │  - Rule 2: Hardcoded True/False                  │   │
│  │  - Rule 3: Missing Consumer Validation           │   │
│  │  - Rule 4: Cross-Language Boundary               │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 3.1 Input

- Path to IDL schema files (`.proto`, `.fbs`, `.capnp`)
- Path to producer source code
- Path to consumer source code
- (Optional) Build files for boundary detection

### 3.2 Output

SARIF (Static Analysis Results Interchange Format) for IDE integration:

```json
{
  "ruleId": "UNAUTHENTICATED-PROVENANCE-001",
  "level": "error",
  "message": {
    "text": "Boolean field 'borrow_check_passed' in VerifiedProgram is set to constant 'true' by producer (analyzer/src/lib.rs:18) but never read by consumer (backend/LLVMEmitter.cpp:64)"
  },
  "locations": [
    {"physicalLocation": {"artifactLocation": "analyzer/src/lib.rs", "region": {"startLine": 18}}},
    {"physicalLocation": {"artifactLocation": "backend/LLVMEmitter.cpp", "region": {"startLine": 64}}}
  ]
}
```

---

## 4. Implementation Phases

### Phase 1: Minimal Viable Detector (Neuro-specific)

**Scope:** Single binary, hardcoded to Neuro's `VerifiedProgram` protobuf
**Language:** Rust (leverages existing `prost`/`tonic` ecosystem)
**Effort:** ~200-300 lines

**Features:**
- Parse `shared_ast/ast.proto`
- Extract boolean fields with semantic names
- Search `analyzer/src/` for field writes
- Search `backend/` for field reads
- Report mismatches

**Limitations:**
- Only works on Neuro codebase
- No cross-language detection
- No build-file analysis

### Phase 2: General Protobuf Detector

**Scope:** Any protobuf-based producer-consumer boundary
**Language:** Rust with `syn` (Rust), `tree-sitter` (C++/Python/Go)
**Effort:** ~500-800 lines

**Features:**
- Configurable IDL schema path
- Language-agnostic source parsing via tree-sitter
- Rule 1 + Rule 3 implemented
- SARIF output

**Limitations:**
- Only protobuf (not FlatBuffers, MessagePack, etc.)
- Requires manual configuration of producer/consumer paths

### Phase 3: Build-System Aware Detector

**Scope:** Cross-language boundary detection from build files
**Language:** Rust
**Effort:** ~1000-1500 lines (depends on build systems supported)

**Features:**
- Parse Cargo.toml, CMakeLists.txt, package.json
- Identify IDL compilation targets
- Auto-detect producer/consumer language pairs
- Rule 4 implemented

**Limitations:**
- Build system diversity makes this fragile
- May require per-project configuration

---

## 5. Evaluation Plan

### 5.1 True Positive Testing

Run detector against Neuro codebase, verify it catches:
- `borrow_check_passed` (should fire Rule 1, 2, 3)
- `type_check_passed` (should fire Rule 1, 2, 3)

### 5.2 False Positive Testing

Run detector against known-immune systems:
- CompCert (no IDL boundary — should fire nothing)
- wasmtime (validation fused — should fire nothing)
- rustc (in-process tracking — should fire nothing)

### 5.3 Cross-Validation

Run detector against known-vulnerable external systems (if source available):
- Docker Notary (Go + protobuf — should detect unread signature fields)
- Kubernetes admission plugins (Go — should detect missing ephemeral container checks)

---

## 6. Limitations and Future Work

### What This Detector Does NOT Catch

1. **Environment variable opt-in patterns** (Docker DCT) — requires runtime analysis
2. **Missing CLI commands** (npm audit signatures) — requires workflow analysis
3. **Missing code paths** (K8s subresources) — requires API surface analysis
4. **Cryptographic bypasses** (go-tuf threshold=0) — requires semantic analysis of verification logic

### Future Extensions

- **FlatBuffers/MessagePack support:** Extend IDL parser to other serialization formats
- **CI/CD integration:** Run detector in CI pipelines to catch regressions
- **Fix suggestions:** Auto-generate code fixes (e.g., "add `if (!verified.borrow_check_passed()) return false;`")
- **SARIF ecosystem:** Integrate with GitHub Code Scanning, VS Code, etc.

---

## 7. Dependencies

- `prost` (protobuf parsing) — already in Neuro's dependency tree
- `tree-sitter` (source parsing) — for cross-language support in Phase 2
- `serde_sarif` (SARIF output) — for IDE integration

---

## 8. Success Criteria

1. **Neuro detection:** Detector flags `borrow_check_passed` and `type_check_passed` as unread semantic booleans
2. **No false positives on immune systems:** Detector produces zero findings for CompCert, wasmtime, rustc
3. **Actionable output:** Findings include file:line references for both producer and consumer
4. **SARIF compliance:** Output can be consumed by GitHub Code Scanning

---

## 9. Out of Scope

This detector is **not** a universal supply-chain security scanner. It does not:
- Verify cryptographic signatures
- Check for known CVEs
- Analyze runtime behavior
- Detect TOCTOU races
- Audit CI/CD pipelines

It is a **static analysis tool for a specific class of IDL-mediated provenance
boundary vulnerabilities** — the class that affects Neuro and similar
multi-language compilation pipelines.
