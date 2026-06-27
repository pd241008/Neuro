
<h1 align="center">🧠 NEURO</h1>

<p align="center">
  <strong>The Zero-Trust, Memory-Safe Compiler Pipeline</strong>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Orchestrator-Rust-orange.svg?style=flat-square&logo=rust" alt="Rust"></a>
  <a href="https://dotnet.microsoft.com/en-us/languages/csharp"><img src="https://img.shields.io/badge/Frontend-C%23-239120.svg?style=flat-square&logo=c-sharp" alt="C#"></a>
  <a href="https://isocpp.org/"><img src="https://img.shields.io/badge/Backend-C%2B%2B-00599C.svg?style=flat-square&logo=c%2B%2B" alt="C++"></a>
  <a href="https://en.wikipedia.org/wiki/C_(programming_language)"><img src="https://img.shields.io/badge/Runtime-C-A8B9CC.svg?style=flat-square&logo=c" alt="C"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License"></a>
</p>

---

## 🚀 Overview

**NEURO** is a blazing-fast, memory-safe, polyglot Compiler Pipeline built with **Rust**, **C#**, **C++**, and **C**. Designed with a **"security-first"** philosophy, NEURO acts as a strict validator for execution-critical software, ensuring memory safety and logical consistency at the language level.

### Why NEURO?

*   **⚡ World-Class DX**: Visually stunning terminal interfaces and precise, tutor-like error reporting using `miette`.
*   **🛡️ Zero-Trust Middle-End**: Mathematically guarantees logic and memory safety before a single line of target code is generated.
*   **🔒 Domain-Specific Constraints**: Eliminates general-purpose vulnerabilities by strictly defining expression boundaries.

---

## 🏗️ System Architecture

The pipeline is divided into three isolated micro-architectures compiled into a single workspace.

```mermaid
graph LR
    A[Source Code] --> B(neuro_cli - Rust)
    B --> C(frontend - C#)
    subgraph frontend [The Lexer & Parser]
        C --> C1[Lexing]
        C1 --> C2[Parsing]
        C2 --> C3[Raw AST]
    end
    C3 --> D(analyzer - Rust)
    subgraph analyzer [The Auditor]
        D --> D1[Security Audit]
        D1 --> D2[Verified AST]
    end
    D2 --> E(backend - C++)
    subgraph backend [The Translator]
        E --> E1[Lowering]
        E1 --> E2[LLVM IR]
    end
    E2 --> F[Executable]
    F -.-> G(runtime - C)
    
    style frontend fill:#1e1e1e,stroke:#239120,stroke-width:2px
    style analyzer fill:#1e1e1e,stroke:#f74c00,stroke-width:2px
    style backend fill:#1e1e1e,stroke:#00599c,stroke-width:2px
```

### Modules

1.  **`neuro_cli` (The Orchestrator) [Rust]**: Handles file I/O, progress visuals, multi-threading, and triggers the compilation phases.
2.  **`frontend` (The Lexer & Parser) [C#]**: Responsible for lexical analysis, hand-written recursive descent parsing, and initial error reporting. Generates the initial Protobuf AST.
3.  **`analyzer` (The Security Auditor) [Rust]**: Performs the Zero-Trust Security Audit, memory-safety checks, and AST validation.
4.  **`backend` (The Translator) [C++]**: Ingests the proven AST and lowers it into highly optimized target code (LLVM IR).
5.  **`runtime` (The Foundation) [C]**: Provides low-level memory management and I/O primitives for the generated executables.
6.  **`compiler` (Legacy C Foundation) [C]**: A Lex/Yacc based foundational compiler engine for prototyping and legacy support.

---

## 🛠️ Getting Started

### Prerequisites

*   **Rust**: Latest stable version (via `rustup`)
*   **.NET SDK**: 8.0+ (for the C# Frontend)
*   **CMake & C++ Compiler**: GCC/Clang/MSVC (for the C++ Backend)
*   **LLVM**: Development headers (required by the backend)
*   **Protobuf Compiler**: `protoc` (for shared AST definitions)

### Installation

```bash
# Clone the repository
git clone https://github.com/pd241008/neuro-compiler.git
cd neuro-compiler

# Build the project
cargo build --release
```

### Usage

```bash
./target/release/neuro compile example.nro
```

---

## 🗺️ Development Roadmap

### Phase 1: Architecture & Scaffolding `[COMPLETED ✅]`
- [x] Scaffold Rust Workspace.
- [x] Wire crate dependencies.
- [x] Cyberpunk-style terminal visuals (`clap`, `indicatif`).

### Phase 2: Domain-Specific Language (DSL) `[COMPLETED ✅]`
- [x] **Syntax Design**: Mapping keywords (`fn`, `let`, `mut`, `type`).
- [x] **Grammar Specification**: Formal EBNF rules.
- [x] **Security Rules**: Defining rejection criteria for unsafe patterns.

### Phase 3: The Front-End (`frontend`) `[COMPLETED ✅]`
- [x] **The Lexer**: High-performance C# memory-scanner with offset tracking.
- [x] **The Parser**: Hand-written recursive descent parser outputting Protobuf AST.
- [x] **DX Integration**: Structure errors for `miette` reporting in Rust.

### Phase 4: Zero-Trust Middle-End `[COMPLETED ✅]`

#### Sub-Phase 4.1: Symbol Table Implementation `[COMPLETED ✅]`
- [x] Scope stack management (`push_scope` / `pop_scope`) for block-level scoping.
- [x] Variable insertion with type, mutability, and initialization tracking.
- [x] Variable lookup across current and parent scopes.
- [x] Offset tracking per variable for backend code generation.

#### Sub-Phase 4.2: Protobuf AST Integration `[COMPLETED ✅]`
- [x] Generate Rust types from `shared_ast/ast.proto` (prost) in shared workspace crate.
- [x] Implement AST deserialization in `audit_ast()`.
- [x] Define the AST visitor/traversal interface in `semantic_analysis.rs`.

#### Sub-Phase 4.3: Semantic Analysis — Type System & Expression Validation `[COMPLETED ✅]`
- [x] Recursive AST traversal for all statement/expression nodes.
- [x] Type resolution for literals, variables, and operations.
- [x] Strict type mismatch detection (no implicit coercion — e.g., `float` → `int`).
- [x] Function return type validation against declared return type.

#### Sub-Phase 4.4: Security Auditor — Borrow Checker Integration `[COMPLETED ✅]`
- [x] Variable move semantics (read/write after move detected).
- [x] Scope-aware variable state tracking.
- [x] 10 integration tests.

#### Sub-Phase 4.5: Analyzer Pipeline & Error Reporting `[COMPLETED ✅]`
- [x] Structured `NeuroError` diagnostics with `miette`.
- [x] Verified AST written to disk (not discarded).
- [x] Pipeline phases wired (frontend → analyze → backend → link).

### Phase 5: The Back-End (`backend`) `[COMPLETED ✅]`

#### Sub-Phase 5.1: Proto Extension & AST Enrichment `[COMPLETED ✅]`
- [x] Extend `ast.proto` with resolved type annotations.
- [x] Collect and attach type metadata during semantic analysis.
- [x] Write enriched AST for C++ backend consumption.

#### Sub-Phase 5.2: C++ Backend Ingestion & CLI Wiring `[COMPLETED ✅]`
- [x] Parse enriched Protobuf AST in `backend/main.cpp`.
- [x] Wire `LLVMEmitter` entry point.
- [x] Invoke backend from Rust CLI pipeline.

#### Sub-Phase 5.3: LLVM IR Lowering — Data & Arithmetic `[COMPLETED ✅]`
- [x] Functions → LLVM function declarations.
- [x] Variables → `alloca` + `store`, assignments → `load` + `store`.
- [x] Binary arithmetic → LLVM arithmetic instructions.
- [x] Comparisons → LLVM `icmp` / `fcmp`.

#### Sub-Phase 5.4: LLVM IR Lowering — Control Flow & I/O `[COMPLETED ✅]`
- [x] `IfStmt` / `WhileStmt` → `cmp` + `br`.
- [x] Function calls → `call` instruction.
- [x] `Printf`/`Scanf` → external `libc` calls.

#### Sub-Phase 5.5: Linking & Final Binary `[COMPLETED ✅]`
- [x] Build runtime library from `io.c` and `memory.c`.
- [x] Compile `.ll` → `.o`, link with runtime → executable.
- [x] Clean up intermediate files on success.

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📜 License

Distributed under the MIT License. See `LICENSE` for more information.

<p align="center">
  Built with 🦀 and ☕ by the Prathmesh Desai
</p>
