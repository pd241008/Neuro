# Target B Audit — Wasmtime v17.0.1 Precompiled Modules (.cwasm)

## Environment
- `wasmtime-cli 17.0.1 (601e229d7 2024-02-07)` installed at `.tools/bin/wasmtime`
- Source tree (for line-number citations): `results/audit_wasmtime/wasmtime-src` @ tag `v17.0.1`, commit `601e229d7ad8d98ce388d7bd0535734d846fcba5`

## Commands and outputs

### 1. Compile a minimal module to AOT
```
$ wasmtime compile add.wasm -o add.cwasm
# add.wasm: 41 bytes (hand-crafted minimal `add(i32,i32)->i32` module)
# add.cwasm: 13800 bytes ELF
```

### 2. Load the precompiled artifact directly (AOT deserialize path)
```
$ wasmtime run --allow-precompiled add.cwasm
exit=0
```
The CLI did NOT re-run the WebAssembly verifier; it deserialized the precompiled
ELF directly.

### 3. Tamper the native machine code (`.text`)
```
readelf -S add.cwasm  → .text PROGBITS, file offset 0x1000, size 0x1000, flags AX
$ flip byte at file offset 0x1100 (0x00 -> 0xFF)
$ wasmtime run --allow-precompiled add_tampered.cwasm
exit=0          ← ACCEPTED. No integrity check over compiled code.
```

### 4. Tamper the engine metadata (`.wasmtime.engine`) — control experiment
```
$ flip first bincode Metadata byte at file offset 0x48 (0x18 -> 0xE7)
$ wasmtime run --allow-precompiled add_tampered_engine.cwasm
Error: io error:      ← REJECTED. bincode::deserializefailed on Metadata.
```
Wasmtime validates *environment metadata* but performs no content-integrity
(MAC/Hash) check over the *machine code* itself.

## Source citations (wasmtime v17.0.1)

- `crates/wasmtime/src/module.rs:523` — `pub unsafe fn deserialize_file(&Engine, path)`
  delegates to `engine.load_code_file(path, ObjectKind::Module)` then
  `Module::from_parts(engine, code, None)`.
- `crates/wasmtime/src/module.rs:495` — `pub unsafe fn deserialize` and both safety comments:
  > "This function is marked as `unsafe` because if fed invalid input or used
  > improperly this could lead to memory safety vulnerabilities. ... Arbitrary
  > input could, for example, replace valid compiled code with any other valid
  > compiled code, meaning that this can trivially be used to execute arbitrary
  > code otherwise."
  > "the contents of the file are not validated to be a valid precompiled module."
- `crates/wasmtime/src/engine.rs:615-632` — `load_code_file` →
  `load_code(MmapVec::from_file(path), expected)`; `load_code` calls
  `serialization::check_compatible` then `CodeMemory::new(mmap)?; code.publish();`
- `crates/wasmtime/src/engine/serialization.rs:75` — `pub fn check_compatible`:
  validates ONLY the ELF header (ELFOSABI_WASMTIME + e_flags), the wasmtime
  version string, and `bincode::deserialize::<Metadata>::check_compatible` (compiler
  features: target, ISA extension, pooling strategy, etc.). There is **no hash,
  digest or MAC over the compiled machine code content**.
- `module.rs:~450-465` — during serialization `engine.append_compiler_info` and
  `append_bti` are appended "only used during deserialization" — again feature
  metadata, not content attestation.

## Finding (for Section 6.1.1)
Wasmtime's `.cwasm` consumer path (`Module::deserialize_file` →
`Engine::load_code_file` → `check_compatible`) re-validates only *compatible
engine/compiler metadata*, exactly Cranelift's compile-time type/bounds decisions
are trusted and the emitted machine code is not authenticated on load. Contrast
with solc (`--import-ast` re-runs full semantic analysis, see `../audit_solc/`).

## Line numbers
- deserialize_file fn signature: module.rs line 523
- deserialize fn signature: module.rs line 495
- load_code_file: engine.rs line 615
- check_compatible: engine/serialization.rs line 75
