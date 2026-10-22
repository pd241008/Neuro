#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# Task 4 demo — REAL containers (Docker in WSL2).
#
# Three actual containers with a shared Docker volume as /workspace:
#
#   1) analyzer  (image bastion-demo-tools):  OWNS BASTION_SIGNING_KEY,
#                builds the payload generator and writes the signed artifact.
#   2) poisoner  (image bastion-demo-tools):  NO signing key in env (exit 99
#                guard if one leaks), overwrites /workspace/output.verified.ast
#                with the unsigned exploit payload.
#   3) backend   (image debian:bookworm-slim): consumes the artifact.
#                  3a pre-fix backend (commit c480981) → exit 0 + use-after-move IR
#                  3b fixed backend   → exit 1 + "HMAC signature verification failed"
#                The backend binary is dynamically linked against host libs, so
#                the container invokes it through the read-only mounted host
#                loader — the container provides the isolation boundary while
#                the host runtime provides the ABI.
#
# Outputs: results/ci_demo_docker/*.log
# Requires: dockerd running + `docker build -t bastion-demo-tools ci_demo` done.
# ─────────────────────────────────────────────────────────────────────────────
set -u

PROJ="$(cd "$(dirname "$0")/.." && pwd)"
LOGS=$PROJ/results/ci_demo_docker
VOL=bastion-demo-ws
IMG=bastion-demo-tools
KEY="ci-demo-prod-signing-key-do-not-leak"
CARGO_VOL=bastion-demo-cargo-home
TGT_VOL=bastion-demo-cargo-target
PREFIX_BIN=/tmp/bastion-prefix/backend/build/neuro_backend
FIXED_BIN=$PROJ/backend/build/neuro_backend

rm -rf "$LOGS"; mkdir -p "$LOGS"
log() { echo -e "$@" | tee -a "$LOGS/demo_transcript.log"; }

if ! docker info >/dev/null 2>&1; then
  echo "dockerd not running — start: rm -f /var/run/docker.pid && dockerd (see DEMO.md)" >&2
  exit 2
fi
docker image inspect "$IMG" >/dev/null 2>&1 || docker build -q -t "$IMG" "$PROJ/ci_demo" >/dev/null
if [ ! -x "$PREFIX_BIN" ]; then
  echo "pre-fix backend missing — rebuild from worktree c480981 (see ci_demo/DEMO.md)" >&2
  exit 2
fi

docker volume rm "$VOL" >/dev/null 2>&1
docker volume create "$VOL" >/dev/null

log "═══════════════════════════════════════════════════════════════════"
log " CI/CD 2-container attack demo — REAL DOCKER CONTAINERS — $(date)"
log " image:    $IMG   volume: $VOL (the shared /workspace)"
log "═══════════════════════════════════════════════════════════════════"
log ""

fail=0

# Shared cargo caches so the poisoner doesn't rebuild from scratch.
docker volume create "$CARGO_VOL" >/dev/null 2>&1
docker volume create "$TGT_VOL"   >/dev/null 2>&1

# Payload generator: the host-built target/release/examples/ci_payload is
# mounted read-only via /src and executed in the container (same glibc, since
# the tools image is built from this same host's packages).
PAYLOAD_BIN=/src/target/release/examples/ci_payload

# ── Step 1: analyzer container (owns the key) ──────────────────────────────
log "── Step 1: ANALYZER container (has BASTION_SIGNING_KEY) ──"
docker run --rm \
  --name ci-demo-analyzer \
  -v "$PROJ":/src:ro \
  -v "$VOL":/workspace \
  -e BASTION_SIGNING_KEY="$KEY" \
  "$IMG" bash -c "
    $PAYLOAD_BIN normal /workspace/output.verified.ast
  " >"$LOGS/step1_analyzer.log" 2>&1
RC=$?
NB=$(docker run --rm -v "$VOL":/ws alpine:latest sh -c 'wc -c < /ws/output.verified.ast 2>/dev/null' 2>/dev/null || echo 0)
log "   container exit: $RC — /workspace/output.verified.ast: ${NB:-0} bytes"
if [ "${NB:-0}" -gt 0 ]; then
  log "   signed artifact written"
else
  log "   FAILED — see results/ci_demo_docker/step1_analyzer.log"; fail=1
fi
log ""

# ── Step 2: poisoner container (NO key — that is the point) ───────────────
log "── Step 2: POISONER container (no BASTION_SIGNING_KEY anywhere in env) ──"
docker run --rm \
  --name ci-demo-poisoner \
  -v "$PROJ":/src:ro \
  -v "$VOL":/workspace \
  -w /src "$IMG" bash -c "
    if env | grep -q '^BASTION_SIGNING_KEY='; then echo 'FATAL: signing key leaked into poisoner env'; exit 99; fi
    $PAYLOAD_BIN exploit /workspace/output.verified.ast
  " >"$LOGS/step2_poisoner.log" 2>&1
RC=$?
bytecount=$(docker run --rm -v "$VOL":/ws alpine:latest sh -c 'wc -c < /ws/output.verified.ast 2>/dev/null' 2>/dev/null || echo 0)
log "   container exit: $RC — poisoned artifact: ${bytecount:-0} bytes"
if grep -q "FATAL" "$LOGS/step2_poisoner.log" 2>/dev/null; then
  log "   KEY LEAK DETECTED — aborting review"; fail=1
fi
if [ "${bytecount:-0}" -gt 0 ]; then
  log "   /workspace/output.verified.ast overwritten with the UNSIGNED exploit payload"
else
  log "   FAILED — no artifact present"; fail=1
fi
log ""

# ── Step 3a: backend container — PRE-FIX backend ───────────────────────────
log "── Step 3a: BACKEND container (pre-fix build, commit c480981, no HMAC) ──"
docker run --rm \
  --name ci-demo-backend-prefix \
  -v /usr:/host-usr:ro \
  -v "$PREFIX_BIN":/opt/neuro_backend:ro \
  -v "$VOL":/workspace \
  debian:bookworm-slim \
  /host-usr/lib/ld-linux-x86-64.so.2 --library-path /host-usr/lib \
      /opt/neuro_backend /workspace/output.verified.ast /workspace/prefix_out.ll \
  >"$LOGS/step3a_prefix.stdout.log" 2>"$LOGS/step3a_prefix.stderr.log"
RC=$?
defines=$(docker run --rm -v "$VOL":/ws alpine:latest sh -c 'grep -c "^define" /ws/prefix_out.ll 2>/dev/null' 2>/dev/null)
log "   container exit: $RC — 'define' directives in emitted IR: ${defines:-0}"
if [ "$RC" -eq 0 ] && [ "${defines:-0}" -gt 0 ]; then
  log "   EMITTED use-after-move LLVM IR (POC) → results/ci_demo_docker/step3a_prefix_ir.log"
  docker run --rm -v "$VOL":/ws alpine:latest cat /ws/prefix_out.ll > "$LOGS/step3a_prefix_ir.log"
else
  log "   UNEXPECTED — pre-fix backend should exit 0 and emit IR"; fail=1
  cp "$LOGS/step3a_prefix.stderr.log" "$LOGS/step3a_prefix_ir.log" 2>/dev/null
fi
log ""

# ── Step 3b: backend container — FIXED backend ─────────────────────────────
log "── Step 3b: BACKEND container (fixed: CRYPTO_memcmp + boolean gate) ──"
docker run --rm \
  --name ci-demo-backend-fixed \
  -v /usr:/host-usr:ro \
  -v "$FIXED_BIN":/opt/neuro_backend:ro \
  -v "$VOL":/workspace \
  -e NEURO_SIGNING_KEY="$KEY" \
  debian:bookworm-slim \
  /host-usr/lib/ld-linux-x86-64.so.2 --library-path /host-usr/lib \
      /opt/neuro_backend /workspace/output.verified.ast /workspace/fixed_out.ll \
  >"$LOGS/step3b_fixed.stdout.log" 2>"$LOGS/step3b_fixed.stderr.log"
RC=$?
log "   container exit: $RC"
if [ "$RC" -ne 0 ] && grep -q "HMAC signature verification failed" "$LOGS/step3b_fixed.stderr.log"; then
  log "   ABORTED: $(head -1 "$LOGS/step3b_fixed.stderr.log")"
elif [ "$RC" -eq 0 ]; then
  log "   UNEXPECTED — fixed backend must NOT accept the unsigned poisoned artifact"; fail=1
else
  log "   rejected with unexpected error:"; log "     $(head -3 "$LOGS/step3b_fixed.stderr.log")"; fail=1
fi
log ""

# ── Control: honest signed artifact against the fixed backend ─────────────
log "── Control: fixed backend on a freshly signed (honest) artifact ──"
HONEST=bastion-demo-honest
docker volume create "$HONEST" >/dev/null
docker run --rm \
  -v "$PROJ":/src:ro -v "$HONEST":/workspace \
  -e BASTION_SIGNING_KEY="$KEY" "$IMG" bash -c "
    $PAYLOAD_BIN normal /workspace/honest.verified.ast
  " >>"$LOGS/step1_analyzer.log" 2>&1
docker run --rm \
  -v /usr:/host-usr:ro \
  -v "$FIXED_BIN":/opt/neuro_backend:ro \
  -v "$HONEST":/workspace \
  -e NEURO_SIGNING_KEY="$KEY" \
  debian:bookworm-slim \
  /host-usr/lib/ld-linux-x86-64.so.2 --library-path /host-usr/lib \
      /opt/neuro_backend /workspace/honest.verified.ast /workspace/honest_out.ll \
  >"$LOGS/control_honest.stdout.log" 2>"$LOGS/control_honest.stderr.log"
RC=$?
log "   container exit: $RC"
[ "$RC" -ne 0 ] && { log "   UNEXPECTED — signed artifact should be accepted"; fail=1; }
log ""

docker volume rm "$VOL" "$HONEST" "$TGT_VOL" "$CARGO_VOL" >/dev/null 2>&1

log "═══════════════════════════════════════════════════════════════════"
log "ATTACK SIMULATION COMPLETE (real containers)"
if [ "$fail" -eq 0 ]; then
  log "RESULT: ✓ reproduced in genuine Docker containers —"
  log "        ✓ poisoner had NO signing key, still overwrote the artifact;"
  log "        ✓ pre-fix backend compiled the exploit (exit 0, use-after-move IR);"
  log "        ✓ fixed backend aborted it (exit 1: HMAC signature verification failed);"
  log "        ✓ honest signed artifact accepted (exit 0)."
else
  log "RESULT: ✗ FAILED — inspect results/ci_demo_docker/*.log"
fi
log "═══════════════════════════════════════════════════════════════════"
exit "$fail"
