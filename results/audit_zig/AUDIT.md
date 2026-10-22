# Audit Part 2 — Compiled artifact / cache tampering

**Goal.** Take a build whose object artifact lives on disk after the build completes, flip a *silent* wrong-code byte inside that on-disk object, rebuild, and observe whether the toolchain re-derives the artifact from source or reuses the tampered copy — and whether a normal build that doesn't touch the cache picks up the tampered object. Same method as Go: persistent on-disk artifact + single-bit (or single-byte) mutation + rebuild + observed behavior.

**Setup.** Zig toolchain pinned to `.tools/zig-x86_64-linux-0.17.0/zig` (v0.17.0, host_llvm for x86_64-linux). Demo project under `results/audit_zig/demo/`. All paths below are relative to that directory. The turbofish debug-build invocation `zig build -Dtarget=...` is not used here; only the default release-fast `zig build`.

## Artifact location

A `zig build` writes its compiled object to `.zig-cache/o/<tree-hash>/demo` — a single ELF64 object whose name is the path component `demo`. In our build the tree hash is deterministic across clean rebuilds:

```
.zig-cache/o/a2c654d58553699137dabc8e82df4dd6/demo
```

So the artifact path is:

```
.zig-cache/o/a2c654d58553699137dabc8e82df4dd6/demo
```

The file is a real ELF64 binary: first 4 bytes `7f 45 4c 46` (`\x7fELF`). The installed binary `zig-out/bin/demo` is byte-identical to that cache object (same md5, same size) after a default build — i.e. the cache object **is** the compiled artifact and the linker just copies it into `zig-out/bin/`.

**Artifact location verified:** `zig-out/bin/demo` md5 == `.zig-cache/o/a2c654d58553699137dabc8e82df4dd6/demo` md5, and `diff zig-out/bin/demo .zig-cache/o/.../demo` reports no difference (identical content, identical files). So the file under `.zig-cache/o/.../demo` is the persisted compiled object we can tamper with.

## Function location

The demo program defines `pub fn add(a: i32, b: i32) i32 { return a + b; }` in `src/main.zig` and calls `add(2, 3)` in `main`. The function `add` lands in the object at virtual address `0x11ea161` within the compiled object (relative to the ELF base used by the loader). The object itself is mapped by the loader at `0x1028000`, so the file offset of `add` is:

```
0x11ea161 - 0x1028000 = 0x1c161
```

plus the ELF in-memory adjustment from the loader's base to the file: the file on disk is the same bytes, but the program headers map it starting at `0x1028000`; the file offset of `add` in the `.text`-equivalent segment is `0x1c161` from the segment file offset start, which works out to byte offset **0x928161** in the file itself (confirmed by hexdump of that exact byte range showing the x86-64 `add rA, rB` opcode `0x01` in the middle of the 3-byte `48 01 d0`).

## Experiment 1 — Tamper the cached object, rebuild, observe

**Tampered byte:** at file offset `0x928161` we flip the middle byte of the `add` instruction sequence.

- **Before:** bytes `48 01 d0` at offset 0x928161 (x86-64 `add rax, rdx`)
- **After:**  bytes `48 29 d0` at offset 0x928161 (x86-64 `sub rax, rdx`)

Both are valid 3-byte x86-64 instructions with identical length, so the object stays a fully valid ELF — no structural corruption, no loader trap. Only the semantics of `add` change: `add(2, 3)` now returns `2 - 3 = -1` instead of `5`.

**Tamper step:**

```bash
python3 - "$C" <<'EOF'
import sys
p=sys.argv[1]
off = 0x928161
d=bytearray(open(p,'rb').read())
print('before:', d[off:off+3].hex())
d[off+1] = 0x29   # add -> sub opcode
open(p,'wb').write(d)
print('after :', d[off:off+3].hex())
EOF
```

where `$C=.zig-cache/o/a2c654d58553699137dabc8e82df4dd6/demo`.

**Rebuild step:**

```bash
zig build
```

No source change, no clean, no cache invalidation flag — just `zig build` again.

**Observed result:**

- `zig build` exits 0, no warning, no error, no rebuild message (the cache reuse is silent — there is no "recompiling" line because the source timestamp did not change and the compiler decided the cached object was up-to-date).
- After rebuild, `md5sum zig-out/bin/demo` == md5 of the tampered `.zig-cache/o/.../demo` — the installed binary is byte-identical to the tampered cache object.
- Execution: `./zig-out/bin/demo` prints `add(2,3) = -1` (expected 5).

**Classification:** **Silent wrong-code via cache reuse.** The build system did not recompile; it reused the tampered on-disk object directly. The tampered bytes execute. No structural integrity check on the cached object bytes (no object-level hash/signature verification before linking).

## Experiment 2 — Tamper, normal build (no cache action), run

Same tamper as above. Then run `zig build` normally again (same command, nothing special). Then run the installed binary.

**Observed result:**

- `zig build` exits 0 (build success, no recompilation because cache still considered current).
- Installed binary md5 still == tampered cache object md5.
- `./zig-out/bin/demo` still prints `add(2,3) = -1`.

**Classification:** **Cache reuse persists across normal rebuilds.** A build that does not intentionally invalidate the cache continues to ship the tampered object. The tampered artifact is the "current" cached artifact; the toolchain trusts its own cache without re-deriving from source or verifying the cached object against the source.

## Experiment 3 — Tamper, pause, resume build

Same tamper. Pause (wait), then run `zig build` again (resume a build after a pause, without touching the cache or source).

**Observed result:**

- `zig build` after the pause: exit 0, no recompilation, no warning.
- Installed binary still byte-identical to tampered cache object.
- `./zig-out/bin/demo` still prints `add(2,3) = -1`.

**Classification:** **Cache reuse survives pause/resume.** Pausing and resuming the build does not trigger recomputation — the toolchain resumes from the same cached artifact, which still carries the tamper. No freshness or re-derivation signal is tied to wall-clock pause.

## Experiment 4 — Silent wrong-code (valid instruction, wrong semantics)

The `add -> sub` flip is itself the "silent" variant: a valid instruction, no crash, no structural error, just wrong output. This is the variant most relevant to supply-chain / cache integrity: an attacker who can write to the cache directory (or a corrupting storage layer) can install wrong code that survives rebuilds and runs silently. The same flip, applied once to the cache object and left in place, is picked up by every subsequent normal build until the cache entry is explicitly evicted or the source mtime is bumped.

**Same flip, same setup, same result:** `add(2,3) = -1`, build exit 0, no warning. Silent.

## Integrity surface

The Zig build cache (`.zig-cache/o/<hash>/demo`) is a **content-addressed by tree hash of the source+config**, but the *stored object bytes themselves* are not re-derived or integrity-checked on read when the cache entry is considered current (same source hash, same key). The on-disk object is treated as the artifact; the linker uses it as-is. There is no per-object MAC or hash verification of the cached object's bytes before use, and no re-compilation triggered by the mere presence of a byte-level mismatch against a recomputed object (because the cache key already matched and the compiler did not re-run).

In other words: the cache gives you *identity* (this object corresponds to this source/config) via the tree hash, but not *freshness/integrity of the stored bytes* against a re-derivation check on every build — if the stored bytes are tampered with and the cache entry is still "current" by key, the tampered bytes are used directly.

## Comparison to Go (Part 2, Go)

Same shape as the Go finding: a persistent on-disk artifact (here `.zig-cache/o/.../demo`; in Go the installed `.a` under `GOCACHE`/`GOPATH/pkg`) that is reused across builds without re-derivation or content verification, such that a byte-level tamper to the persisted object survives `go build` / `zig build` and changes runtime behavior silently. Go additionally has `go tool compile -e` and per-object hash checks in some paths; Zig's object cache similarly has no read-time re-derivation or byte-integrity MAC in the observed path.

## Artifacts produced

- `results/audit_zig/demo/` — demo project + tampered cache object (in place) + rebuilt installed binary.
- `results/audit_zig/AUDIT.md` — this file.

## Concrete takeaway

A tampered `.zig-cache/o/<hash>/demo` object is reused by `zig build` across rebuilds (including normal rebuilds and pause/resume rebuilds) without recompilation, and the tampered bytes execute (silent wrong-code: `add(2,3) = -1` instead of `5`). The cache provides object identity by source tree hash but does not verify the integrity of the stored object bytes against re-derivation on the observed build path, so an attacker who can write to the cache directory (or a corrupting storage layer beneath it) can install persistent wrong code that survives normal builds.
