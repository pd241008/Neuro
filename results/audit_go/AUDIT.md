# Audit: Go 1.27.3 build cache (GOCACHE content-hash validation)

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
