# Task 4 — CI/CD 2-Container Attack Demo (verified)

Docker is not available in this environment, so the verification of record ran
through `ci_demo/run_demo.sh` (local 3-step emulation with isolated directories
for each "container" plus a shared workspace). The same steps are packaged as
`ci_demo/docker-compose.yml` and `.github/workflows/ci-demo.yml` for the paper.

## Steps executed (2026-10-22, full transcript: `results/ci_demo/demo_transcript.log`)

| Step | action | result |
|---|---|---|
| 1 | analyzer container (owns `BASTION_SIGNING_KEY`) | wrote signed `output.verified.ast` (129 bytes) |
| 2 | poisoner (NO key in env) | overwrote artifact with unsigned use-after-move payload, 147 bytes (32-byte dummy HMAC) |
| 3a | **pre-fix** backend (built from commit `c480981` — no HMAC check) | **exit 0** — emitted the use-after-move LLVM IR (`results/ci_demo/step3a_prefix_ir.log`) |
| 3b | **fixed** backend (`CRYPTO_memcmp` + boolean gate) | **exit 1** — `Error: Backend emission failed: HMAC signature verification failed` |
| Ctl | fixed backend on the honest step-1 artifact | exit 0 — legitimate pipeline unaffected |

Key isolation: the poisoner's environment (`env -u BASTION_SIGNING_KEY -u NEURO_SIGNING_KEY`)
never contains a signing key — write access to `output.verified.ast` alone was
enough for the pre-fix compromise; the fixed backend rejects it cryptographically.

## Files

- `run_demo.sh` — verification of record (dockerless)
- `docker-compose.yml` — 2-container compose version
- `../.github/workflows/ci-demo.yml` — GitHub Actions version (artifacts as the shared volume)
- `../tests/examples/ci_payload.rs` — payload generator (`normal` / `exploit` modes)
- `../results/ci_demo/*.log` — raw step outputs incl. the emitted exploit IR
