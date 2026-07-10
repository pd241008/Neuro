# Finding: Unauthenticated Provenance Boundary in the Neuro Compiler Pipeline

**ID:** NEURO-2026-001
**Severity:** High (Design-Level)
**Component:** Analyzer-to-Backend Boundary (`analyzer/src/lib.rs` → `*.verified.ast` → `backend/main.cpp`)
**Date:** 2026-07-10
**Status:** Confirmed — empirically verified with reproducible test suite
**Reporter:** Boundary test suite (`tests/boundary_tests.rs`, 7/7 passing)

---

## Abstract

The Neuro compiler pipeline establishes safety properties — type correctness,
absence of use-after-move, and absence of illegal concurrent borrows — in the
Rust analyzer (`analyzer/src/lib.rs`), serializes them as boolean metadata
inside a `VerifiedProgram` protobuf (`shared_ast/ast.proto:21–25`), and delivers
the result to the C++ LLVM backend as an intermediate file. However, the backend
(`backend/main.cpp:28–34`, `backend/LLVMEmitter.cpp:64`) accepts this file based
solely on successful protobuf deserialization and `has_program()` — it never
inspects the safety flags, and no cryptographic binding ties the file's contents
to the analyzer's authorization.

A hand-crafted protobuf satisfying only the wire schema is sufficient to produce
valid LLVM IR that violates borrow-check invariants, using the honest,
unmodified backend binary — no compiler bugs, fuzzing, or memory unsafety
required. This finding demonstrates that provenance metadata in multi-language
compilation pipelines must be *consumed and verified* by downstream stages, not
merely *produced and annotated* by upstream ones.

---

## 1. System Overview

The Neuro compiler pipeline has four phases, orchestrated by
`neuro_cli/src/main.rs:161–339`:

```
Phase 1          Phase 2              Phase 3                  Phase 4
[Frontend (C#)] → output.ast → [Analyzer (Rust)] → output.verified.ast → [Backend (C++) output.ll] → [Linker]
```

- **Phase 1 (Frontend):** The C# lexer/parser (`frontend/`) reads `.nro` source
  and emits a serialized `Program` protobuf to `output.ast`.
- **Phase 2 (Analyzer):** The Rust analyzer (`analyzer/`) reads the `Program`,
  runs type checking (`semantic_analysis.rs`) and borrow checking
  (`borrow_check.rs`), and wraps the enriched result in a `VerifiedProgram`
  protobuf written to `output.verified.ast`.
- **Phase 3 (Backend):** The C++ backend (`backend/`) reads the `VerifiedProgram`
  from disk and lowers it to LLVM IR.
- **Phase 4 (Linker):** LLVM IR is compiled to a native binary via `clang`.

The trust boundary under examination is the file-system handoff between Phase 2
and Phase 3: the `output.verified.ast` file.

---

## 2. Formal Finding Statement

**Name:** Unauthenticated Provenance Boundary

**Definition:** A *provenance boundary* exists whenever a safety-relevant
metadata artifact crosses a process, language, or serialization boundary and the
consumer does not independently verify the artifact's authenticity or contents.

### 2.1 What Property Fails

The correctness invariant for the pipeline is:

> Let *A* be the set of programs that pass the analyzer's audit (type-check and
> borrow-check). Let *B* be the set of programs that the backend is willing to
> compile to LLVM IR. Correctness requires *B ⊆ A*.

Currently, *B* is the set of all well-formed `VerifiedProgram` protobufs
containing a `Program` message — regardless of the values of
`borrow_check_passed` or `type_check_passed`. Since an attacker can construct a
`VerifiedProgram` with any `Program` payload and `borrow_check_passed: true`,
*B* is a strict superset of *A*. The invariant *B ⊆ A* is violated.

### 2.2 Why It Fails: Rejection is Fail-Closed; Acceptance is Unauthenticated

- **Rejection is fail-closed.** If the analyzer detects a violation,
  `audit_ast()` (`analyzer/src/lib.rs:11–12`) returns `Err`, and no
  `.verified.ast` file is written (`neuro_cli/src/main.rs:230–238`). The
  pipeline halts. This path is sound.

- **Acceptance is unauthenticated.** There is no mechanism that proves a given
  `.verified.ast` file was produced by the analyzer as opposed to constructed
  independently. The `VerifiedProgram` message's `borrow_check_passed` and
  `type_check_passed` fields (`shared_ast/ast.proto:23–24`) are metadata
  *produced* by the analyzer but never *consumed* by the backend. A forged file
  carrying `borrow_check_passed: true` is indistinguishable from an authentic
  one.

### 2.3 Structural Evidence (Source Code)

**The analyzer always sets both flags to `true`:**

```rust
// analyzer/src/lib.rs:16-20
let verified = VerifiedProgram {
    program: Some(program),
    borrow_check_passed: true,   // Always true; never false, anywhere
    type_check_passed: true,     // Always true; never false, anywhere
};
```

No assignment to `false` exists anywhere in the codebase. The flags exist only
as documentation of intent — they are structurally incapable of carrying
adverse information.

**The backend never reads these flags:**

```cpp
// backend/main.cpp:24-28
neuro::ast::VerifiedProgram verified;
if (!verified.ParseFromIstream(&input)) { ... }

// backend/LLVMEmitter.cpp:64-68
bool LLVMEmitter::emitProgram(const VerifiedProgram& verified) {
    if (!verified.has_program()) {   // <-- Only check
        error_ = "VerifiedProgram missing Program";
        return false;
    }
    // ... proceeds directly to codegen, never touches borrow_check_passed or type_check_passed
```

The only structural check is `has_program()`. The boolean fields are parsed from
the wire but never inspected by any code path.

**No cryptographic binding exists:**

The handoff between analyzer and backend is a raw protobuf file on disk
(`neuro_cli/src/main.rs:230–238`). No signature, HMAC, hash, or checksum is
computed or verified at any stage. A case-insensitive search for `hmac`, `sha`,
`md5`, `checksum`, `integrity`, and `tamper` across all `.rs`, `.cpp`, `.h`,
and `.proto` files returns zero hits. Hits for `hash` and `sign` are exclusively
protobuf-generated `.GetHashCode()` equality methods and the `Assign` token in
the C# lexer — neither related to cryptographic integrity.

---

## 3. Attack Scenario: Reproducible Counterexample

### 3.1 Threat Model

- **Attacker capability:** Filesystem write access to the `.verified.ast` output
  path. This is achievable via build-script injection, compromised CI step,
  shared build directory, or any process running under the same user.
- **Attacker goal:** Produce valid LLVM IR from a program that violates a
  borrow-check invariant, using the honest and unmodified `neuro_backend` binary.
- **Constraints:** No fuzzing, no exploitation of compiler bugs, no modification
  to any source file in the repository.

### 3.2 Borrow-Check Rule to Violate

The borrow checker (`analyzer/src/borrow_check.rs:62–65`) enforces:

```
check_read(var):
    if state[var] == Moved:
        return Err("Use of moved value `{var}`")
```

This prevents use-after-move — reading a variable after its value has been
transferred to a function by value. In the backend, violating this would produce
code that reads from a location whose value has logically been consumed, leading
to undefined behavior at runtime.

### 3.3 The Exploit: Hand-Crafted `VerifiedProgram`

The following 110-byte protobuf was constructed by hand, serialized, and fed to
the unmodified `neuro_backend` binary:

```
Input (hex): 0a680a047465737412360a046d61696e1a020804220f120d0a0178
12001a041202082a200122103a0e320c0a03666f6f12051a030a017822073a051
a030a017812120a03666f6f12070a0361726712001a02080412140a036261721
2090a03617267120208021a02080410011801
```

This decodes to the following logical structure (Rust pseudocode):

```rust
VerifiedProgram {
    program: Some(Program {
        name: "test",
        functions: vec![
            // main: declares x=42, moves x into foo(x), then reads x
            Function {
                name: "main",
                return_type: VOID,
                body: vec![
                    Declaration { name: "x", type: INT, initializer: Literal(42) },
                    ExpressionStmt(Call { function: "foo", args: [Variable("x")] }),
                    ExpressionStmt(Variable("x")),  // <-- USE-AFTER-MOVE
                ],
            },
            Function { name: "foo", params: [Int], return_type: VOID, body: [] },
            Function { name: "bar", params: [Bool], return_type: VOID, body: [] },
        ],
    }),
    borrow_check_passed: true,   // Forged
    type_check_passed: true,     // Forged
}
```

**Critical:** `audit_ast()` was never called. The program was constructed
directly as a `VerifiedProgram`, skipping Phase 2 entirely.

### 3.4 Result

The backend exits with code 0 and produces the following LLVM IR:

```llvm
; NEURO Compiler — LLVM IR Output
; Module: test
target triple = "x86_64-pc-linux-gnu"

declare i32 @printf(i8*, ...)
declare i32 @scanf(i8*, ...)

define void @main() {
entry:
  %x = alloca i32
  %r0 = add i32 0, 42       ; x = 42
  store i32 %r0, ptr %x
  %r2 = load i32, ptr %x    ; load x (move into call)
  %r1 = call void @_foo(i32 %r2)  ; foo(x) — x is now moved
  %r3 = load i32, ptr %x    ; USE-AFTER-MOVE: reads x again
  ret void
}

define void @_foo(i32) { ... }
define void @_bar(i1) { ... }
```

Line `%r3 = load i32, ptr %x` reads `x` after it was moved into `@_foo`. The
borrow checker would have rejected this program with `"Use of moved value 'x'"`,
but the backend compiled it without question.

### 3.5 Flags Are Read by Nothing

A second variant was tested with `borrow_check_passed: false` and
`type_check_passed: false` on the identical malformed program. The backend
produces **byte-identical LLVM IR** and exits with the same success code. This
proves the flags are not just untrusted when set to `true` — they are read by
*nothing*. The `VerifiedProgram` wrapper is purely decorative.

---

## 4. Severity Assessment

### 4.1 What This Is Not

This is **not** a soundness bug in the Rust analyzer's borrow checker or type
checker. The analyzer correctly rejects violating programs. The borrow checker
(`borrow_check.rs`) and semantic analysis (`semantic_analysis.rs`) are
structurally sound in isolation.

### 4.2 What This Is

The severity is that **the analyzer's correctness is unenforceable by the
backend.** The guarantee produced by the analyzer exists only within its own
process and does not propagate through the file-system-mediated trust boundary.
The `VerifiedProgram` wrapper is, in effect, a *decorative attestation* — it
asserts provenance but does not prove it.

The practical consequence: any attacker with write access to the build
directory can produce programs that violate memory safety properties (use-after-
move, illegal concurrent borrows) and have them compiled to valid LLVM IR by the
unmodified backend. This can lead to undefined behavior, memory corruption, or
exploitable vulnerabilities in the compiled output.

### 4.3 Generalization: Comparative Analysis

This finding generalizes to any multi-stage compilation or verification pipeline
where:

1. A trusted analysis pass produces a proof-carrying intermediate
   representation.
2. The IR crosses a language, process, or serialization boundary.
3. The downstream consumer trusts the IR *because of what it represents*
   (provenance metadata) rather than *verifying what it contains*.

We systematically audited nine additional real-world systems spanning verified
compilers, package registries, container signing, and admission control. The
results show that the vulnerability is not universal — it correlates with a
specific architectural choice: whether the system **couples** validation into
execution (immune) or **decouples** them into producer and consumer roles
separated by an optionally-verified boundary (vulnerable).

#### Comparative Table

| System | Producer | Consumer | Boundary Type | Classification | Evidence |
|--------|----------|----------|---------------|----------------|----------|
| **Neuro compiler** | Rust analyzer sets `borrow_check_passed=true` in protobuf | C++ backend ignores flags, checks only `has_program()` | Cross-binary protobuf (unauthenticated) | **VULNERABLE** | `analyzer/src/lib.rs:16-22`, `backend/LLVMEmitter.cpp:64` |
| **Docker Content Trust** | `docker trust sign` creates Notary signatures | `docker pull` does not verify by default; `--disable-content-trust` bypasses | Cross-tool opt-in signature | **VULNERABLE** | Docker docs (DCT disabled by default); CVE-2026-23992 (go-tuf threshold=0, CVSS 5.9) |
| **npm provenance** | `npm publish --provenance` generates SLSA L3 attestation | `npm install` does not verify; `npm audit signatures` opt-in | Registry-to-client attestation | **VULNERABLE** | npm RFC 0049; CVE-2026-45321 (CVSS 9.6, valid SLSA on malicious packages) |
| **PyPI PEP 740** | Warehouse verifies at upload, exposes via Integrity API | `pip install` does not verify; requires external tools | Index-to-client attestation | **VULNERABLE** | Trail of Bits blog (2024-11-14); warehouse#15871 (closed, index-side complete, pip integration separate) |
| **K8s admission** | ImagePolicyWebhook validates containers | Ephemeral containers bypass webhook entirely | In-process subresource gap | **VULNERABLE** | CVE-2023-2727, CVE-2023-2728 |
| **K8s Gatekeeper** | Webhook configuration matches `resources: ['*']` | Subresources (`*/scale`, `pods/ephemeralcontainers`) not matched | Webhook configuration gap | **VULNERABLE** | gatekeeper#1837 (closed, fixed v3.9.0 via PR #2054); gatekeeper-library#188 |
| **in-toto/SLSA** | `slsa-github-generator` creates attestations | `slsa-verifier` verifies correctly when invoked | CI attestation verification | **MITIGATED** | Tools correct; deployment misconfiguration |
| **wasmtime** | Validation fused into `Module::from_binary()` | No separate consumer — same function | Fused validation-compilation | **STRUCTURALLY-IMMUNE** | `crates/cranelift/src/module.rs` |
| **CompCert** | 21 Coq passes in single `TotalTransform` proof | No serialization between passes | In-process proof composition | **STRUCTURALLY-IMMUNE** | `driver/Compiler.v` |
| **CakeML** | End-to-end HOL4 correctness theorem | No intermediate serialization | Monolithic verified function | **STRUCTURALLY-IMMUNE** | HOL4 proof |
| **rustc** | `MirPhase` enum enforced via `assert!()` in pass manager | Codegen accesses `Body<'tcx>` via query in same process | In-process struct field | **NOT-APPLICABLE** | `compiler/rustc_middle/src/mir/syntax.rs`, `pass_manager.rs` |

#### Distribution Analysis

The distribution across these eleven systems reveals a clear architectural
correlate: the "unauthenticated provenance boundary" vulnerability appears
exclusively in systems with **separable produce-then-trust boundaries** — where
a safety or security attestation is generated by one component (analyzer, signer,
validator) and consumed by a separate component (backend, runtime, installer) with
no cryptographic binding or structural enforcement between them.

**Vulnerable systems share three properties:**

1. **Serialization boundary:** The attestation crosses a process, language, or
   file-system boundary (protobuf in Neuro, signatures in Docker, registry
   responses in npm/PyPI, webhook configurations in Kubernetes).

2. **Optional verification:** The consumer's verification is either not
   implemented (Neuro backend never reads flags), disabled by default (Docker
   DCT, npm provenance), or opt-in (PyPI requires external tools).

3. **Decoupled producer and consumer:** The producer and consumer are separate
   binaries, tools, or code paths that can evolve independently, creating
   divergence between what is asserted and what is checked.

**Immune systems avoid this by eliminating the boundary:**

- **CompCert** and **CakeML** compose all passes inside a single proof
  obligation — no serialization point exists where a "type_check_passed" boolean
  could be forged because no such boolean exists. The proof *is* the attestation.
- **wasmtime** calls the validator inside `Module::from_binary()` before
  compilation; the validated module never exists as a deserializable artifact
  with a checkable flag.
- **rustc** tracks phase via in-process `MirPhase` enum enforced by `assert!()`;
  the phase never crosses a serialization boundary where an external consumer
  would need to trust it.

**The design lesson** is not "add signatures" but rather "eliminate the
boundary" — or, if a boundary is architecturally necessary, ensure it is
*structurally enforced* (type-system guarantees, proof-carrying code) rather than
*optionally verified* (flags, environment variables, opt-in audit commands). The
pattern recurs specifically in ad-hoc multi-language pipelines (Neuro) and
deployment/supply-chain tooling (Docker, npm, K8s) where the producer and
consumer were designed independently and the attestation format was an
afterthought.

---

## 5. Empirical Verification (Test Suite)

All claims in this document were verified by an automated test suite
(`tests/boundary_tests.rs`, 7 tests, 7 passing) against the unmodified codebase.

### 5.1 Stage 1 — Baseline (Fail-Closed Rejection)

| Test | Input | Expected | Observed | Verdict |
|------|-------|----------|----------|---------|
| `stage_1a_valid_program_survives_full_pipeline` | Valid program (no violations) through `audit_ast()` → backend | Backend exits 0, emits `define` in IR | Backend exits 0, emits `define void @main` | PASS |
| `stage_1b_use_after_move_rejected_by_analyzer` | Use-after-move program through `audit_ast()` | `audit_ast()` returns `Err` | `Err(AnalysisError("Use of moved value 'x'"))` | PASS |

**Conclusion:** The fail-closed path is sound. When programs pass through the
analyzer, violating programs are correctly rejected. No `VerifiedProgram` bytes
are ever produced for a program that fails audit.

### 5.2 Stage 2 — Bypass / Exploit Reproduction (Core Finding)

| Test | Input | Expected | Observed | Verdict |
|------|-------|----------|----------|---------|
| `stage_2_bypass_use_after_move_flags_true` | Hand-crafted `VerifiedProgram` (no `audit_ast()`), `borrow_check_passed=true, type_check_passed=true` | Backend exits 0, emits IR | Backend exits 0, IR contains `define void @main` with `load i32, ptr %x` after move | PASS |
| `stage_2_bypass_use_after_move_flags_false` | Same program, `borrow_check_passed=false, type_check_passed=false` | Backend exits 0, emits IR (flags unchecked) | Backend exits 0, **byte-identical IR** to flags-true variant | PASS |

**Conclusion:** The provenance boundary is unauthenticated. A hand-crafted
protobuf bypasses all safety checks. The flags are decorative metadata — they
are parsed from the wire but never inspected by any code path. The backend
accepts the same malformed program regardless of flag values.

**Artifact evidence:** The complete 110-byte input protobuf (hex-encoded) and
the backend's full IR output are recorded in
`results/stage2_flags_true.log` and
`results/stage2_flags_false.log`.

### 5.3 Additional Observation — Backend Lacks Structural Validation (Stage 3)

> **Editorial note:** This stage supports the main finding but is a distinct
> claim ("the backend has no input validation") rather than part of the core
> finding ("provenance metadata is unauthenticated"). It is included for
> completeness; for a tight paper, this can be collapsed into 1–2 sentences in
> the Discussion or moved to an appendix with the artifact logs.

The backend's only structural check is `has_program()`. Three tests confirm
that malformed or incomplete protobuf fields produce silent defaults or
corrupted IR rather than rejections:

| Test | Input | Observed |
|------|-------|----------|
| Unset `resolved_type` | `Expression` with no `resolved_type` | Backend silently defaults to `i32` via `typeToLLVM()` `default` case |
| Unset `stmt_kind` | `Statement` with no oneof variant | Silently skipped (`default: break;` in `emitStatement`) |
| Unset `expr_kind` | `Expression` with no oneof variant | Emits `%rN = add i32 0, 0` as no-op fallback |

These are silent-corruption paths, not rejection paths. The backend is a pure
code generator — it trusts whatever protobuf it receives and performs no
defensive validation of structural integrity.

---

## 6. Proposed Remediations

### Fix 1: Backend Checks the Boolean Flags (Minimal — ~10 lines)

**Change:** In `LLVMEmitter.cpp`, add immediately after `has_program()`:

```cpp
if (!verified.borrow_check_passed() || !verified.type_check_passed()) {
    error_ = "VerifiedProgram failed safety checks. Refusing to emit.";
    return false;
}
```

**Cost:** Negligible. Single-file change, no new dependencies.

**Effectiveness:** Closes the gap against *accidental* misuse (e.g., a
developer passing a raw `.ast` file to the backend). Does **not** close the gap
against *adversarial* tampering — the attacker can set both flags to `true`.

**Trade-off:** Trivial to implement and zero runtime cost. Provides
defense-in-depth against honest mistakes but is not a security boundary.

### Fix 2: HMAC-Sign the Serialized `VerifiedProgram` (Intermediate — ~40 lines)

**Change:**

1. Add `bytes signature = 4;` to the `VerifiedProgram` message in `ast.proto`.
2. In `analyzer/src/lib.rs`, after constructing the `VerifiedProgram` (but
   before attaching the signature), serialize all fields except `signature`,
   compute `HMAC-SHA256(serialized_bytes, secret_key)`, and store the result in
   `signature`. The key is read from an environment variable or compiled-in
   constant.
3. In `LLVMEmitter.cpp`, before codegen, serialize the received protobuf's
   non-signature fields, recompute the HMAC, and compare to the received
   `signature`. Refuse to emit on mismatch.

**Cost:** Moderate. Requires a cryptographic library on the C++ side (e.g.,
OpenSSL's `HMAC()` or a header-only library like `picohash`). Key management
must be addressed.

**Effectiveness:** Provides **authentication** — the backend can verify that a
given `.verified.ast` was produced by an entity holding the HMAC key. The
attacker cannot forge a valid signature without the key.

**Trade-off:** Key management is the limiting factor. If both analyzer and
backend run on the same machine under the same user, the key file is as
accessible as the `.verified.ast` file, limiting the threat model to cross-user
or cross-machine attacks. For CI/CD pipelines where the analyzer and backend run
in different containers or under different service accounts, this is a
meaningful improvement.

### Fix 3: Re-derive Safety Properties in the Backend (Strongest — ~200+ lines)

**Change:** Implement a minimal type-checker and move-checker in C++ that
re-analyzes the `Program` extracted from the `VerifiedProgram` before emitting
LLVM IR:

- **Type consistency:** Walk all `VariableDeclaration` and `Assignment` nodes;
  verify that every `resolved_type` is consistent with the declared type and the
  initializer's type.
- **Move tracking:** Walk all `Expression` nodes referencing variables; track
  `VariableState::Moved` per-scope (mirroring `borrow_check.rs`); reject if any
  variable is referenced after a move.
- **Borrow tracking (optional extension):** Maintain a map of active borrows
  per variable; reject writes to borrowed variables and reads of
  exclusively-borrowed variables.

**Cost:** Significant. Requires re-implementing core analyzer logic in C++.
Introduces a maintenance burden where changes to the Rust analyzer must be
mirrored in the C++ backend.

**Effectiveness:** Makes the backend a **true consumer of the safety proof**
rather than a passive recipient. Eliminates the trust dependency on the analyzer
entirely — even a compromised or buggy analyzer cannot cause the backend to
compile an unsafe program.

**Trade-off:** The risk of divergence — if the two implementations disagree,
you get false rejections (conservative, annoying but safe) or, worse, false
acceptance (defeating the purpose). This is appropriate only if the threat model
requires the backend to be independently verifiable (e.g., the backend is
provided by a different vendor, or runs in a less trusted environment than the
analyzer).

### Recommended Combination

| Layer | Fix | Addresses |
|-------|-----|-----------|
| Sanity gate | Fix 1 (boolean check) | Accidental misuse; developer ergonomics |
| Security boundary | Fix 2 (HMAC auth) | Adversarial tampering; cross-user/cross-CI attacks |
| Defense-in-depth | Fix 3 (re-derivation) | Compromised analyzer; backend independence |

---

## 7. Revised Abstract / Contribution Framing

> We demonstrate that the safety guarantees established by the Rust
> borrow-checking and type-checking analyzer in the Neuro compiler do not survive
> serialization into the `VerifiedProgram` protobuf: the C++ LLVM backend reads
> this intermediate representation without inspecting the `borrow_check_passed`
> or `type_check_passed` fields, and the file-system handoff between analyzer and
> backend carries no cryptographic binding. A hand-crafted protobuf satisfying
> only the wire schema is sufficient to produce valid LLVM IR that violates
> borrow-check invariants — no compiler bugs, fuzzing, or memory unsafety
> required. This finding motivates the general principle that provenance metadata
> in multi-language compilation pipelines must be *consumed and verified* by
> downstream stages, not merely *produced and annotated* by upstream ones, and we
> propose three remediation strategies at increasing assurance levels:
> boolean-field checking, HMAC-authenticated handoff, and backend-local
> re-derivation of safety properties.

---

## Appendix A: Key Source Locations

| File | Lines | Relevance |
|------|-------|-----------|
| `analyzer/src/lib.rs` | 16–20 | `audit_ast()` constructs `VerifiedProgram` with hardcoded `true` flags |
| `analyzer/src/borrow_check.rs` | 62–65 | `check_read()` — use-after-move detection (the invariant the backend fails to enforce) |
| `shared_ast/ast.proto` | 21–25 | `VerifiedProgram` schema — the decorative attestation fields |
| `backend/main.cpp` | 24–28 | Backend parses `VerifiedProgram`, never checks flags |
| `backend/LLVMEmitter.cpp` | 64–68 | `emitProgram()` — only checks `has_program()` |
| `backend/LLVMEmitter.cpp` | 53–62 | `typeToLLVM()` — silent default to `i32` for unset types |
| `backend/LLVMEmitter.cpp` | 234–236 | `emitStatement` default — silently skips unset `stmt_kind` |
| `backend/LLVMEmitter.cpp` | 446–448 | `emitExpression` default — emits `add i32 0, 0` for unset `expr_kind` |
| `neuro_cli/src/main.rs` | 227–232 | Pipeline orchestration — the file-system handoff |

## Appendix B: Reproduction Steps

```bash
# 1. Build the backend (if not already built)
cd backend && make && cd ..

# 2. Run the boundary test suite
cargo test --test boundary -- --nocapture

# 3. Inspect the artifact logs
cat results/stage2_flags_true.log
cat results/stage2_flags_false.log
```

## Appendix C: Artifact Inventory

| Artifact File | Stage | Contents |
|---------------|-------|----------|
| `results/stage1a_valid.log` | 1 — Baseline | Valid program: 98-byte input, exit 0, IR with `define i32 @main` |
| `results/stage2_flags_true.log` | 2 — Bypass | Use-after-move with `flags=true`: 110-byte input, exit 0, IR with use-after-move |
| `results/stage2_flags_false.log` | 2 — Bypass | Use-after-move with `flags=false`: 106-byte input, exit 0, **identical IR** |
| `results/stage3a_unset_type.log` | 3 — Field Drift | Unset `resolved_type`: 56-byte input, exit 0, silent `i32` default |
| `results/stage3b_unset_stmt.log` | 3 — Field Drift | Unset `stmt_kind`: 58-byte input, exit 0, statement silently skipped |
| `results/stage3b_unset_expr.log` | 3 — Field Drift | Unset `expr_kind`: 60-byte input, exit 0, `add i32 0, 0` fallback |

All artifact logs contain the full hex-encoded input protobuf, backend exit
code, stderr, and emitted LLVM IR, formatted for direct inclusion in a paper
appendix.
