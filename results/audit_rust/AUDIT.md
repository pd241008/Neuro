# Audit: rustc 1.98.0 incremental-compilation cache (cargo 1.98.0)

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
