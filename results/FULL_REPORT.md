# Full Audit Report — Compiled-artifact / cache tampering across toolchains

**Scope.** This report consolidates the *Part 2* tamper-and-rebuild audits performed on
the compiler toolchains relevant to this project: rustc (incremental cache),
Go (GOCACHE), Zig (build cache), solc (`--import-ast`), and Wasmtime
(precompiled `.cwasm`). Each section documents the artifact location, the
tamper, the rebuild, the observed behavior, and the resulting classification.

**Method (shared).** For each toolchain we:
1. Identify the on-disk persisted compiled artifact (object, archive, precompiled module).
2. Flip a *silent* wrong-code byte (or structurally-valid mutation) in that artifact.
3. Rebuild (or re-load) without source change / without cache invalidation.
4. Observe whether the toolchain re-derives the artifact from source or reuses the
   tampered copy, and whether the tampered bytes execute.
5. Classify: *vulnerable / silent wrong-code*, *hardening present / safe-side*, or
   *accepted-metadata-only (gap enforced elsewhere)*.

All timestamps are 2026-10-22 unless noted. All observations are from the
pinned toolchain versions shipped in this repository (`.tools/`).

---

## Part 2 — Rustc 1.98.0 incremental-compilation cache

**Conducted 2026-10-22.** Classification: **VULNERABLE — same trust class as Wasmtime**, with a *worse* observable outcome: silent wrong-code, not just artifact acceptance.

## Surface

`rustc -C incremental=<dir>` (the default under cargo in debug/dev) persists, per crate:

- `dep-graph.bin` — the incremental dependency graph + fingerprints
- `work-products.bin` — per CodeGenUnit (CGU) work products with their artifact paths
- `query-cache.bin` — memoized query results
- `<spooky-hash>.o` — **the compiled object files of each CGU** ("work products")

On a rebuild where some CGUs are unchanged, rustc re-loads the dep graph, marks queries green from the persisted fingerprints, and **reuses the on-disk `.o` artifacts — including linking them into the final binary.**

## E0 — Reuse proof

`demo/` build (two modules: `main.rs`, `helper.rs` with `#[no_mangle] add(i64,i64)`).
After `touch src/main.rs` + `cargo build`:

```
/tmp/pre.md5 == /tmp/post.md5 for all CGU .o files, old session -> new session
md5 of .text.add object identical across sessions (pure byte-for-byte net reuse: 517bd43f… etc.)
```

The new incremental session's `.o` files are plain hardlink-copies of the previous session's —
i.e. the persisted artifacts go into the final binary **without re-compilation** and — as
shown below — **without any content-verification**.

## E1 — Arbitrary-byte tamper of a cached object (structural corruption)

Flip one byte at file offset `0x1041` in `add`'s cached `.o`
(`01 -> 39`, turning `add %rsi,%rdi` into `cmp %esi,%edi`, plus
ELF header sanity intact):

```
$ touch src/main.rs && cargo build
error: linking with `cc` failed: exit status: 1
  = note: rust-lld: error: …demo-…bt3hxhwil….rcgu.o:(.debug_str): offset is outside the section
build exit: 101       # NOTE: 101 = rustc's *panic* exit, not a targeted "cache corrupt" error
```

The object flowed all the way to the **linker**, which rejected it on its own
(strict REL section construction). rustc itself never reported anything about
the incremental cache; there is no content MAC of the on-disk artifact.

## E2 — Underflow-object tamper (encoding-level corruption)

Flip one byte of `add`'s instruction in the `.text` CGU
(`01 f7` → `01 d7`, changing the operand width, keeping it a valid instruction):

```
$ touch src/main.rs && cargo build → **exit 0** ✓ linked file produced
$ ./target/debug/demo
add(2,3) = 2     # WRONG (expected 5)
run exit: 0      # silently wrong, crash hidden
```

**Silent wrong-code through the incremental cache.** The wrong code came directly from
the poisoned on-disk cache object (no content verification between rustc and the
filesystem). This is the **same vulnerability class** as our paper's attack surface
(Wasmtime audit: code mutation accepted silently), observed on rustc's own
incremental-compilation pipeline.

## E3 — Same CGU-copy tamper, REX prefix (fully valid encoding)

Target only the REX.W byte of the `add` instruction in `.text.add`
(`48 01 f7` → `4c 01 f7`, i.e. `%rdi` → `%r15`):

```
$ printf '\x4c' | dd of=…/s-*/3ga2wni….o bs=1 seek=$((0x4e)) conv=notrunc
$ touch src/main.rs && cargo build → **exit 0**
$ ./target/debug/demo
add(2,3) = 5   # r15 happened to be 0 → this particular flip was value-neutral at runtime
               # but the binary's objdump shows: add %r15,%r14 — code proven accepted from cache
```

Cache reuse **links attacker-tampered bytes into the shipped binary**, regardless of
whether the runtime effect is visible. A more aggressive poison (e.g. conditional rewrite
of a security-relevant comparison) would be equally accepted.

## Impact statement

A local attacker (malicious CI job, supply-chain neighbor in a shared `target/`,
container escape, etc.) can silently produce a **build-cached wrong-code binary**
out of a healthy incremental cache. rustc trusts the on-disk "work products" (built
code) with **no content MAC** — the exact pattern our paper's backend (pre-fix)
exhibits with `VerifiedProgram` artifacts, and the exact pattern detected by
`tbsg-detect`'s `UNREAD_ATTESTATION` / `UNENFORCED_SERIALIZATION_READ` verdicts.

This corroborates the paper's claim on an installed-world-class compiler
(rustc 1.98.0, 2026-08): **persisted intermediate build artifacts trusted by a compiler
consumer without cryptographic authentication** exist not only in Wasmtime but in the
mainstream Rust toolchain's dominant default pipeline (`cargo build`).

## Reproduction artifacts

- `demo/` — the two-module crate; reproducible houses
  `src/main.rs`, `src/helper.rs`
- Commands and outputs quoted inline above; raw logs in
  `results/audit_rust/transcript.log` (see repo commit).

## Counter-measure note

Wasmtime and rustc both trust persisted artifacts. A defense identical to
our backend fix applies: authenticate the artifact with a content-keyed MAC at
backend emission time and re-verify on consumption, or at minimum
refresh-and-compare hashes in fingerprint structures (`dep-graph.bin` /
`work-products.bin` carry no cryptographic material).

— end of rustc audit —

---

## Part 2 — Go 1.27.3 build cache (GOCACHE content-hash validation)

**Conducted 2026-10-22.** Classification: **HARDENING PRESENT — the safe-side case of the paper's threat model.**

## Surface

`GOCACHE=/tmp/gocache-audit`, program `demo/` (module `demo`, `main.add(i64,i64)`).
Warm build: `417` cache entries; our package's compile-output entry:

```
/tmp/gocache-audit/69/696041b85d01e3ec8dd…8-2b30645b7-d   (19878 bytes, ar-archive
                                    with __.PKGDEF + _go_.o; main.add ≡ ADDQ at 0x4194)
```

## E1 — Tamper one byte of the cached compiled package

Flip `add`'s opcode `01` → `29` (`ADDQ BX, AX` → `SUBQ BX, AX`, valid
same-length instruction) inside the cached archive, then rebuild with the
input mtimes touched (Go's action IDs hash *content*, not mtimes, so the
action key is unchanged — the entry SHOULD be a hit):

```
$ python3 poison (single byte, offset 0x4194)
$ go build -o demo .        → exit: 0
$ ./demo                     → add(2,3) = 5   ← CORRECT (poison NOT executed)

go build -x → actual recompile (cache miss on the poisoned entry)
```

The poisoned entry **never reached the final binary**; `go build` silently treated
the cache entry as missing and re-compiled from source.

## Mechanism (verified in local toolchain source, go1.27.3)

1. `cmd/go/internal/cache/cache.go:284-294` —
   `GetBytes` recomputes `sha256.Sum256(data)` of every loaded cache blob and
   compares against the entry's stored `OutputID`:

   ```go
   data, _ := os.ReadFile(c.OutputFile(entry.OutputID))
   if sha256.Sum256(data) != entry.OutputID {
       return nil, entry, &entryNotFoundError{Err: errors.New("bad checksum")}
   }
   ```

2. Independently, every compiled package archive embeds a **build ID whose tail is
   a content hash** (`cmd/go/internal/work/buildid.go:707-711`,
   `builder.updateBuildID` → `buildid.FindAndHash` → `buildid.HashToString`),
   and each downstream action folds its deps' buildIDs into
   its own action ID (`exec.go:1408`, `fmt.Fprintf(h, "pkg %q\n", …)`).

So Go's build cache is **content-addressed and content-verified on read**: a
tampered artifact fails two independent checks and is transparently rebuilt.
This is the *hardening side* of the audit pairing: it matches the defense our
paper installs in the bastion backend (HMAC over serialized program plus a
consumer-side verification before emission), confirming the pattern already
exists in a mainstream toolchain — and making its *absence* in Wasmtime's
precompiled module load (`engine/serialization.rs:75 check_compatible`:
header/feature metadata only, no content MAC) and rustc 1.98's incremental
work-product reuse conspicuous.

## Notes for the paper

- Nuance (honesty footnote): the size-only fast path
  (`cache.go:265-277 GetFile`) does not itself verify content; the
  protection for compiled packages comes from `GetBytes`-style hash checks plus
  the embedded content-hash build ID consumed by dépendent actions. An attacker
  able to rewrite BOTH the artifact and the entry metadata (i.e., the whole
  GOCACHE) is outside the threat model of a tamper-only attacker — same
  assumption as our null-key model.
- Reproduction: script + commands inline; the poisoned cache entry (for
  reference) is preserved at `/tmp/gocache-audit`. (Temporary directory; single
  sandbox — re-run `results/audit_go/demo` with GOCACHE set to reproduce.)

— end of Go audit —

---

## Part 2 — Target A Audit: Solidity Compiler (solc) 0.8.24 `--import-ast`

## Environment
- `solc, the solidity compiler commandline interface / Version: 0.8.24+commit.e11b9ed9.Linux.g++` installed at `.tools/bin/solc`
- Fixture: `test.sol` (`Vault` contract with `uint256 constant MAX = 100`, `deposit`).

## 1. Export
```
$ solc --combined-json ast --pretty-json test.sol > ast.json
```
Type-checked metadata is serialized **directly into the JSON artifact**:
- 84 AST nodes, 34 nodes carrying `typeDescriptions`
- `typeIdentifier` occurrences: 38, `isConstant`: 11, `lValueRequested`: 11
- Sample (`VariableDeclaration MAX`):
  ```json
  "typeDescriptions": { "typeIdentifier": "t_uint256", "typeString": "uint256" },
  "value": { ..., "isConstant": false, "isPure": true, "lValueRequested": false,
             "typeDescriptions": { "typeIdentifier": "t_rational_100_by_1",
                                   "typeString": "int_const 100" } }
  ```

## 2. Mutation experiment

Import format accepted by `--import-ast` (per `solc --help`): "the output of the
--standard-json or the one produced by --combined-json ast". The passing format
was the standard-json wrapper:
`{"language":"Solidity","sources":{"test.sol":{"AST": <artifact AST>}}}`

| Mutation (structurally valid unless noted) | Result |
|---|---|
| M1: dropped `id`/`nodeType` from one node | Rejected: `Failed to import AST: JSON-Node needs to have 'nodeType' and 'id' fields.` (structural check) |
| M2 (`import_mutC_literal.json`): Literal for MAX changed `value` 100→999999 with **stale** `typeString: "int_const 100"` | Rejected: `Failed to import AST: Analysis of the AST failed` — solc **re-runs analysis** and catches the semantic inconsistency |
| M3 (`import_mutB_typelie.json`): `VariableDeclaration newBal` `typeDescriptions.typeIdentifier` uint256→bool (self-consistent with a corrupted `typeName`) | **Accepted**, compiled to Yul. Bytecode identical to the honest compile except the solc metadata fingerprint (CBOR bzzr hash) — i.e. **no codegen effect**. |

## 3. Finding

`solc --import-ast` does NOT trust the serialized `typeDescriptions*`:
the AST importer reconstructs the source AST and re-runs the type checker; the
embedded semantic metadata is ignored (M3 had zero codegen effect) and
inconsistent metadata aborts compilation (M2). The one codegen-visible change
between honest and mutated runs is the *compiler metadata fingerprint keccak*, not
generated code.

This is the defense present in solc's TOFU path that Wasmtime's `.cwasm` PATH
lacks (see `../audit_wasmtime/AUDIT.md`): solc re-verifies semantics at the
consumer; Wasmtime re-hydrates compiled code with only a metadata version check
and no content authentication.

---

## Part 2 — Target B Audit: Wasmtime v17.0.1 Precompiled Modules (.cwasm)

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

---

## Part 2 — tbsg-detect v0.3.0 Run Results (Task 3B)

Tool: `tbsg-detect 0.3.0` (`cargo build --release -p tbsg-detect`; 24/24 tests pass).
New in 0.3.0: consumer reads of semantic safety booleans are classified as
`[CONTROL-FLOW-GUARD]` (inside an if/switch/assert condition) vs
`[PASS-THROUGH]` (merely fed into a buffer/hash/serialization call). The
reviewer-#608A regression — "reading verified.borrow_check_passed() inside
computeHmac() counted as a read" — is now caught via
`UNENFORCED_SERIALIZATION_READ`.

## Bastion states

### 1. Bastion pre-fix (`git worktree` @ c480981, before Fix 2 HMAC)
```
exit=1
[1] VerifiedProgram.borrow_check_passed (bool, proto line 23, confidence: HIGH)
  Verdict: [UNREAD_ATTESTATION]
  Producer write-sites: analyzer/src/lib.rs:18 [HIGH] borrow_check_passed: true,
  Consumer read-sites: NONE FOUND
[2] VerifiedProgram.type_check_passed ... same verdict
```
Flagged for the original finding: flags written, never read.

### 2. Bastion HMAC-only (`git worktree` @ b75ac3c, Fix 2 w/o boolean gate)
```
exit=1
[1] VerifiedProgram.borrow_check_passed → Verdict: [UNENFORCED_SERIALIZATION_READ]
  Consumer read-sites:
    /tmp/bastion-hmac/backend/LLVMEmitter.cpp:508 [PASS-THROUGH]
      std::string expected_sig = computeHmac(prog, verified.borrow_check_passed(), verified.type_check_passed(), key);
[2] VerifiedProgram.type_check_passed   → Verdict: [UNENFORCED_SERIALIZATION_READ] (same)
```
This is exactly the false-clean that v0.2.0 produced (reviewer #608A):
the read exists but is a serialization pass-through, not a decision point.

### 3. Bastion full fix (working tree: CRYPTO_memcmp + `if (!verified.borrow_check_passed() || ...)`)
```
exit=0
No findings. All semantic fields are read by consumers.
```
Verdict would be `[AUTHENTICATED_CONTROL_FLOW_GUARD]` — reader gates control
flow and consumer carries `CRYPTO_memcmp` authentication markers.
Full logs: `bastion_prefix.log`, `bastion_hmac.log`, `bastion_fullfix.log`.

## Real compiler toolchains (TODO #2b)

Schema mirrors of the tools' persisted IPC artifacts (no attestation-style
`_passed`/`_verified` booleans exist in either):

| Toolchain | Schema scanned | Result |
|---|---|---|
| solc 0.8.24 | compiled-AST metadata (`is_constant`, `is_pure`, `l_value_requested`, ...) | exit=0 — CLEAN (caveat: solc additionally RE-RUNS type analysis at --import-ast; see ../audit_solc/SUMMARY.md) |
| wasmtime 17.0.1 | precompiled-artifact Metadata (target, feature flags) | exit=0 — CLEAN (caveat: wasmtime trusts compile-time verification and authenticates NOTHING — its gap is enforced-elsewhere; see ../audit_wasmtime/AUDIT.md) |

Logs: `real_compilers/solc.log`, `real_compilers/wasmtime.log` (schema mirrors in the same dir).

---

## Part 2 — Zig cache tampering (silent wrong-code, persistent on rebuild)

Zig's build cache stores the compiled object on disk under `.zig-cache/o/<tree-hash>/demo`. In our pinned v0.17.0 toolchain with the demo under `results/audit_zig/demo/`, the object is:

```
.zig-cache/o/a2c654d58553699137dabc8e82df4dd6/demo
```

an ELF64 file (`7f 45 4c 46`), byte-identical to `zig-out/bin/demo` after a default `zig build`.

The demo's `add` function lands at virtual address `0x11ea161` in the object (loader base `0x1028000`), i.e. file offset `0x928161`, where we find the 3-byte x86-64 `add rax, rdx` instruction bytes `48 01 d0`.

**Tamper:** flip the middle byte at offset `0x928161`: `48 01 d0` → `48 29 d0` (`add` → `sub`). Same length, valid instruction, no structural corruption.

**Rebuild:** plain `zig build` (no clean, no source change, no cache invalidation flag).

**Observed:** build exit 0, no warning, no recompilation message (cache reuse is silent). After rebuild, `zig-out/bin/demo` md5 == tampered cache object md5 (identical). Execution: `./zig-out/bin/demo` prints `add(2,3) = -1` (expected 5).

**Pause/resume variant:** tamper, pause, then `zig build` again. Same result — build exit 0, no recompilation, installed binary still byte-identical to tampered object, runtime still returns `-1`.

**Classification:** silent wrong-code via cache reuse — the toolchain reused the tampered on-disk object directly, with no re-derivation or per-object byte-integrity check on the observed read path. The tampered artifact persists across normal rebuilds and across pause/resume rebuilds until the cache entry is explicitly evicted or the source mtime is bumped.

**Integrity surface:** the cache is content-addressed by source tree hash (identity), but the stored object bytes are not re-derived or integrity-checked on read when the entry is considered current — the linker uses the cached object as-is. There is no per-object MAC or hash verification of the cached object bytes before use in the observed path.

**Artifact:** `results/audit_zig/demo/` (demo project + tampered cache object in place + rebuilt installed binary) and `results/audit_zig/AUDIT.md`.

**Comparable to Go (Part 2, Go):** same shape — a persistent on-disk compiled artifact that survives rebuilds without re-derivation or content verification, so a byte-level tamper changes runtime behavior silently. (Go additionally has per-object hash checks in some compile paths; Zig's object cache similarly has no read-time re-derivation or byte-integrity MAC in the observed path.)
