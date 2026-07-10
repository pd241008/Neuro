# Boundary Test Suite

This suite demonstrates the unauthenticated provenance boundary between the
Rust analyzer (`analyzer/src/lib.rs`) and the C++ LLVM backend
(`backend/LLVMEmitter.cpp`). Each stage verifies a specific claim from the
Finding document (`docs/FINDING-unauthenticated-provenance-boundary.md`).

## Stage 1 — Baseline (Fail-Closed Rejection)

Stage 1 confirms that the existing pipeline correctly rejects programs with
safety violations when they pass through the analyzer. Test 1a runs a valid
program through the full `audit_ast()` → backend pipeline and confirms LLVM IR
is emitted. Test 1b constructs the exact use-after-move pattern from
`borrow_check_tests::move_variable_then_read_rejected` and asserts that
`audit_ast()` returns `Err` — proving that the fail-closed path (rejection) is
sound and no `VerifiedProgram` bytes are ever produced for a violating program.

## Stage 2 — Bypass / Exploit Reproduction (Core Finding)

Stage 2 is the primary evidence for the Finding. It **skips `audit_ast()`
entirely** and directly constructs a `VerifiedProgram` protobuf by hand,
containing the same use-after-move program from Stage 1b. Two variants are
tested: one with `borrow_check_passed=true, type_check_passed=true`, and one
with both flags `false`. In both cases, the unmodified backend binary emits
valid LLVM IR and exits successfully. This proves two things simultaneously:
(1) the backend accepts programs that the analyzer would reject, and (2) the
`borrow_check_passed`/`type_check_passed` fields are read by nothing — they are
decorative metadata, not an enforced contract.

## Stage 3 — Field Drift on resolved_type and Unset Oneofs

Stage 3 characterizes the backend's behavior when protobuf fields are left
unset or malformed, probing whether any implicit validation catches structural
anomalies. Test 3a sends a program where `resolved_type` is absent on all
expressions and confirms the backend silently defaults to `i32` (via
`typeToLLVM()`'s `default` case). Test 3b sends a `Statement` with no
`stmt_kind` oneof variant and confirms it is silently skipped (`default: break`
in `emitStatement`). Test 3c sends an `Expression` with no `expr_kind` and
confirms the backend emits `add i32 0, 0` as a no-op fallback. All three
demonstrate that the backend performs no structural validation beyond
`has_program()` — unknown or missing fields produce silent defaults or
corrupted IR rather than rejections.
