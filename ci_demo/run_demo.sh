#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# Task 4 demo: 2-container CI/CD attack (local 3-step emulation).
#
# Dockerless fallback for ci_demo/docker-compose.yml — reproduces the same
# steps with three isolated "container" directories plus a shared volume:
#
#   Step 1 (Analyzer container):   owns BASTION_SIGNING_KEY, signs a VALID
#                                  program, writes <workspace>/output.verified.ast
#   Step 2 (Untrusted CI Action):  cache poisoner WITHOUT the key overwrites
#                                  <workspace>/output.verified.ast with the
#                                  unsigned use-after-move exploit payload
#   Step 3 (Backend container):    reads <workspace>/output.verified.ast and runs
#                                  the C++ backend twice:
#                                    3a pre-fix backend  (commit c480981, no HMAC):
#                                       expected exit 0 + use-after-move LLVM IR
#                                    3b fixed backend (CRYPTO_memcmp + gate):
#                                       expected exit 1 + "HMAC signature verification failed"
#
# Outputs: results/ci_demo/*.log
# ─────────────────────────────────────────────────────────────────────────────
set -u

PROJ="$(cd "$(dirname "$0")/.." && pwd)"
DEMO=/tmp/ci_demo
WS=$DEMO/workspace          # the shared /workspace volume
LOGS=$PROJ/results/ci_demo
PAYGEN=$PROJ/target/release/examples/ci_payload
PREFIX_BACKEND=/tmp/bastion-prefix/backend/build/neuro_backend
FIXED_BACKEND=$PROJ/backend/build/neuro_backend

rm -rf "$DEMO" && mkdir -p "$DEMO/analyzer" "$DEMO/poisoner" "$DEMO/backend" "$WS"
mkdir -p "$LOGS"

log() { echo -e "$@" | tee -a "$LOGS/demo_transcript.log"; }

log "═══════════════════════════════════════════════════════════════════"
log " CI/CD 2-container attack demo — $(date)"
log " Payload generator: $PAYGEN"
log " Shared volume:     $WS"
log "═══════════════════════════════════════════════════════════════════"
log ""

fail=0

# ── Step 1: Analyzer container (isolated env, owns the key) ───────────────
log "── Step 1: Analyzer container ──"
( cd "$DEMO/analyzer" && \
  BASTION_SIGNING_KEY="ci-demo-prod-signing-key-do-not-leak" \
  "$PAYGEN" normal "$WS/output.verified.ast" ) >>"$LOGS/step1_analyzer.log" 2>&1
if [ ! -f "$WS/output.verified.ast" ]; then
  log "   FAILED — no artifact written"; fail=1
else
  log "   wrote $WS/output.verified.ast ($(wc -c <"$WS/output.verified.ast") bytes, signed)"
fi
log ""

# ── Step 2: Untrusted CI action / cache poisoner (NO key) ─────────────────
log "── Step 2: Untrusted CI action / cache poisoner (no BASTION_SIGNING_KEY) ──"
( cd "$DEMO/poisoner" && env -u BASTION_SIGNING_KEY -u NEURO_SIGNING_KEY \
  "$PAYGEN" exploit "$WS/output.verified.ast" ) >>"$LOGS/step2_poisoner.log" 2>&1
log "   overwrote $WS/output.verified.ast ($(wc -c <"$WS/output.verified.ast") bytes, UNSIGNED)"
log "   attacker key exposure: none — $DEMO/poisoner never sees any signing key"
log ""

# ── Step 3a: Backend container, PRE-FIX backend (no HMAC check) ───────────
log "── Step 3a: pre-fix backend reads poisoned artifact ──"
( cd "$DEMO/backend" && env -u BASTION_SIGNING_KEY -u NEURO_SIGNING_KEY \
  "$PREFIX_BACKEND" "$WS/output.verified.ast" "$DEMO/backend/prefix_out.ll" ) \
  >"$LOGS/step3a_prefix.stdout.log" 2>"$LOGS/step3a_prefix.stderr.log"
RC=$?
log "   exit code: $RC"
if [ "$RC" -eq 0 ] && grep -q "define" "$DEMO/backend/prefix_out.ll" 2>/dev/null; then
  log "   EMITTED use-after-move LLVM IR (poc): see results/ci_demo/step3a_prefix_ir.log"
  cp "$DEMO/backend/prefix_out.ll" "$LOGS/step3a_prefix_ir.log"
else
  log "   UNEXPECTED — pre-fix backend should exit 0 and emit IR"; fail=1
fi
log ""

# ── Step 3b: Backend container, FIXED backend (HMAC + boolean gate) ───────
log "── Step 3b: fixed backend (CRYPTO_memcmp + semantic flag gate) reads poisoned artifact ──"
( cd "$DEMO/backend" && \
  NEURO_SIGNING_KEY="ci-demo-prod-signing-key-do-not-leak" \
  "$FIXED_BACKEND" "$WS/output.verified.ast" "$DEMO/backend/fixed_out.ll" ) \
  >"$LOGS/step3b_fixed.stdout.log" 2>"$LOGS/step3b_fixed.stderr.log"
RC=$?
log "   exit code: $RC"
if [ "$RC" -ne 0 ] && grep -q "HMAC signature verification failed" "$LOGS/step3b_fixed.stderr.log"; then
  log "   ABORTED: $(head -1 "$LOGS/step3b_fixed.stderr.log")"
elif [ "$RC" -ne 0 ]; then
  log "   rejected with different error:"; log "     $(head -3 "$LOGS/step3b_fixed.stderr.log")"
  fail=1
else
  log "   UNEXPECTED — fixed backend must NOT accept the unsigned poisoned artifact"; fail=1
fi
log ""

# ── Control: un-poisoned artifact (honest run) against fixed backend ──────
log "── Control: fixed backend on the STEP-1 (signed) artifact ──"
( cd "$DEMO/analyzer" && \
  BASTION_SIGNING_KEY="ci-demo-prod-signing-key-do-not-leak" \
  "$PAYGEN" normal "$DEMO/workspace_honest.verified.ast" ) >>"$LOGS/step1_analyzer.log" 2>&1
( cd "$DEMO/backend" && \
  NEURO_SIGNING_KEY="ci-demo-prod-signing-key-do-not-leak" \
  "$FIXED_BACKEND" "$DEMO/workspace_honest.verified.ast" "$DEMO/backend/honest_out.ll" ) \
  >"$LOGS/control_honest.stdout.log" 2>"$LOGS/control_honest.stderr.log"
RC=$?
log "   exit code: $RC ($(tail -2 "$LOGS/control_honest.stdout.log" | head -1))"
if [ "$RC" -ne 0 ]; then log "   UNEXPECTED — signed artifact should be accepted"; fail=1; fi
log ""

log "═══════════════════════════════════════════════════════════════════"
log "ATTACK SIMULATION COMPLETE"
if [ $fail -eq 0 ]; then
  log "RESULT: ✓ reproduced — no HMAC → exploit compiles (exit 0);"
  log "        ✓ HMAC + boolean gate → exploit aborts (exit 1: HMAC signature verification failed);"
  log "        ✓ honest signed artifact still accepted (exit 0)."
else
  log "RESULT: ✗ one or more checks FAILED — inspect results/ci_demo/*.log"
fi
log "═══════════════════════════════════════════════════════════════════"
exit $fail
