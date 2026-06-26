# NEURO Compiler -- Progress Tracker

## Phase 1: Architecture & Scaffolding `[COMPLETED]`
- [x] Scaffold Rust Workspace.
- [x] Wire crate dependencies (including `analyzer` in `neuro_cli`).
- [x] Pipeline CLI with progress bars (`clap`, `indicatif`).
- [x] `audit_ast()` wired into CLI pipeline.

## Phase 2: Domain-Specific Language (DSL) `[COMPLETED]`
- [x] **Syntax Design**: Mapping keywords (`fn`, `let`, `mut`, `if`, `while`, `return`).
- [x] **Grammar Specification**: Formal EBNF rules (with `WhileStmt`, `UnaryExpr`, `String` type).
- [x] **Security Rules**: Defining rejection criteria for unsafe patterns.
- [x] **Proto AST**: `VariableDeclaration` includes `is_mutable` field.

## Phase 3: The Front-End (`frontend`) `[COMPLETED]`
- [x] **Lexer**: Full support for identifiers, numbers (int/float), strings, booleans, symbols, comments.
- [x] **Parser**: Recursive descent with operator precedence -- all expressions (binary/unary/calls/parens).
- [x] **Statements**: `let`, `mut`, `if/else`, `while`, `return`, assignment, expression statements.
- [x] **Source Locations**: Token-accurate line/column tracking.
- [x] **DX Integration**: Structure errors for `miette` reporting in Rust.

## Phase 4: Zero-Trust Middle-End `[COMPLETED]`

### Sub-Phase 4.1: Symbol Table Implementation `[COMPLETED]`
Files: `analyzer/src/symbol_table.rs`
- [x] Scope stack management (`push_scope` / `pop_scope`)
- [x] Variable insertion with type, mutability, and initialization tracking
- [x] Variable lookup (current + parent scopes)
- [x] Offset tracking for backend codegen
- [x] `NeuroType` enum aligned with proto `Type.Kind`
- [x] Conversion from/to proto kind (`from_proto_kind`, `to_proto_kind`)
- [x] `pop_scope` returns `Result` preventing global scope pop

### Sub-Phase 4.2: Protobuf AST Integration `[COMPLETED]`
Files: `shared_ast/`, `analyzer/src/lib.rs`
- [x] Create `shared_ast` Rust crate with prost-build for `ast.proto`
- [x] Add `shared_ast` to workspace
- [x] Wire `shared_ast` as dependency in `neuro_cli` and `analyzer`
- [x] Remove proto compilation from `neuro_cli/build.rs`
- [x] Implement `audit_ast(input: &[u8])` with protobuf deserialization
- [x] AST visitor/traversal interface defined in `semantic_analysis.rs`
- [x] `protoc` presence check before build

### Sub-Phase 4.3: Semantic Analysis -- Type System & Expression Validation `[COMPLETED]`
Files: `analyzer/src/semantic_analysis.rs`, `analyzer/src/lib.rs`
- [x] Recursive AST traversal for all statement/expression nodes
- [x] Type resolution for literals, variables, binary/unary ops, function calls
- [x] Strict type mismatch detection (no implicit coercion)
- [x] Function return type validation
- [x] Control flow condition validation (if/while require bool)
- [x] Variable declaration type checking (declared type vs initializer)
- [x] Duplicate variable/parameter detection
- [x] Immutable assignment prevention
- [x] Mutable variable support (via proto `is_mutable` field)
- [x] Assignment marks target as initialized
- [x] 26 integration tests in `tests/` -- all passing

### Sub-Phase 4.4: Security Auditor -- Borrow Checker Integration `[COMPLETED]`
Files: `analyzer/src/semantic_analysis.rs`, `analyzer/src/borrow_check.rs`
- [x] Wire `BorrowChecker` into AST traversal (AnalysisContext.borrow_checker)
- [x] Check reads during variable access (check_read in resolve_expression)
- [x] Check writes during variable access (check_write in visit_assignment)
- [x] Move semantics enforcement (move_variable on function call arguments)
- [x] Declare variable tracking (declare_variable in visit_declaration + param insertion)
- [x] Scope-based borrow expiry (expire_borrow at scope boundaries)
- [x] 10 integration tests in `tests/borrow_check_tests.rs` -- all passing

### Sub-Phase 4.5: Analyzer Pipeline & Error Reporting `[COMPLETED]`
Files: `neuro_cli/src/main.rs`, `analyzer/src/lib.rs`, `analyzer/src/error.rs`
- [x] Connect C# frontend --> deserialize --> analyze --> serialize
- [x] Rich error diagnostics with `miette` (NeuroError struct with miette::Diagnostic derive)
- [x] Write verified AST to `target/neuro_output/output.verified.ast` (was discarded)
- [x] Replace sleep stubs: C++ backend invocation (or placeholder LLVM IR), Clang linking (or skip)
- [x] Audit command fully wired to read, analyze, and write verified output
- [x] All error paths updated from `String` to `NeuroError` across entire analyzer crate

## Phase 5: The Back-End (`backend`) `[IN PROGRESS]`

### Sub-Phase 5.1: Proto Extension & AST Enrichment
Status: `[COMPLETED]`
Files: `shared_ast/ast.proto`, `analyzer/src/lib.rs`, `analyzer/src/semantic_analysis.rs`
- [x] Extend `ast.proto` with annotation fields (`Type resolved_type` on `Expression`, `VariableDeclaration`)
- [x] Collect resolved type info during semantic analysis — `resolve_expression()` now sets `resolved_type` on each `Expression` node
- [x] Build enriched `VerifiedProgram` in `audit_ast()` with type/safety metadata attached (wraps enriched `Program` with `borrow_check_passed` and `type_check_passed` flags)
- [x] Write enriched AST to disk as the verified output for the C++ backend (output is now `VerifiedProgram` encoded bytes)
- [x] Document the enriched AST wire format (see below)

#### Enriched AST Wire Format

After analysis, `audit_ast()` produces a serialized `VerifiedProgram` protobuf message:

```protobuf
message VerifiedProgram {
    Program program = 1;          // The original program with resolved_type fields populated
    bool borrow_check_passed = 2;  // Always true if analysis succeeded
    bool type_check_passed = 3;    // Always true if analysis succeeded
}
```

Within `program`, every `Expression` node has its `resolved_type` field set to the computed type (e.g. `INT`, `FLOAT`, `BOOL`, `STRING`, `VOID`). Every `VariableDeclaration` has its `resolved_type` set to the declared type. This allows the C++ backend to determine the LLVM type of any expression without performing its own type analysis.

Key fields added to existing messages:
- `Expression.resolved_type` (field 7, `Type`) — populated during semantic analysis
- `VariableDeclaration.resolved_type` (field 5, `Type`) — populated during semantic analysis

### Sub-Phase 5.2: C++ Backend Ingestion & CLI Wiring
Status: `[COMPLETED]`
Files: `backend/main.cpp`, `backend/LLVMEmitter.cpp`, `backend/NeuroBackend.h`, `backend/Makefile`, `neuro_cli/src/main.rs`
- [x] Parse enriched `VerifiedProgram` protobuf from CLI arguments in C++ (`backend/main.cpp`) — reads binary protobuf, validates, and passes to emitter
- [x] Wire `neuro_cli` to invoke the C++ backend binary with the enriched AST path — `run_pipeline()` already passes `<verified_ast_path> <output_ll_path>`; backend binary now exists and is used
- [x] Instantiate `LLVMEmitter` and call `emitIR(ast)` entry point — `LLVMEmitter` walks the full AST and emits LLVM IR for all statement/expression types
- [x] Handle backend errors and propagate them to the user via `miette` — non-zero exit codes with stderr messages are caught and reported in the CLI pipeline
- [x] Integration test (`tests/backend_integration_test.rs`) — verifies the full Rust→C++ pipeline: construct Program, run audit_ast, invoke backend binary, verify LLVM IR output

### Sub-Phase 5.3: LLVM IR Lowering — Data & Arithmetic
Status: `[NOT STARTED]`
Files: `backend/LLVMEmitter.cpp`
- [ ] Map `Function` declarations to LLVM function definitions
- [ ] Map `LET` declarations to `alloca` + `store` instructions
- [ ] Map assignment statements to `load` + `store` instructions
- [ ] Map integer/float literals to LLVM constants
- [ ] Map binary arithmetic (ADD, SUB, MUL, DIV) to LLVM arithmetic instructions
- [ ] Map comparison operators (EQ, NEQ, LT, GT, LTE, GTE) to LLVM `icmp`/`fcmp`

### Sub-Phase 5.4: LLVM IR Lowering — Control Flow & I/O
Status: `[COMPLETED]`
Files: `backend/LLVMEmitter.cpp`, `analyzer/src/semantic_analysis.rs`
- [x] Map `IfStmt` to LLVM `cmp` + `br` (true/false/merge labels)
- [x] Map `WhileStmt` to LLVM loop headers + `br` back-edge
- [x] Map `FunctionCall` to LLVM `call` instruction (fixed: args emitted before call, not interleaved)
- [x] Map `Return` to LLVM `ret` instruction with optional value
- [x] Map `print`/`println`/`read` to external libc `printf`/`scanf` calls via LLVM declaration
- [x] Pre-register built-in I/O function signatures in analyzer symbol table
- [x] 1 integration test (`tests/print_read_test.rs`) — all passing

### Sub-Phase 5.5: Linking & Final Binary
Status: `[NOT STARTED]`
Files: `neuro_cli/src/main.rs`, `CMakeLists.txt`
- [ ] Invoke `clang` on generated `.ll` to produce object file
- [ ] Link with runtime library (`runtime/`) for I/O primitives
- [ ] Output final executable binary to `target/neuro_output/output.bin`
- [ ] Clean up intermediate files (`.ll`, `.o`) on success
