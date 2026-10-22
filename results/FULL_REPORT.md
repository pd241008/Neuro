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
