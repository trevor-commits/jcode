#!/usr/bin/env bash
# Recommended local verification before pushing. Mirrors CI jobs that are safe to
# run on a laptop without provider credentials or network (except cargo fetch).
#
# Usage:
#   scripts/verify_local.sh                 # default pre-push path
#   scripts/verify_local.sh --offline       # no rustc compile (ratchets + python)
#   scripts/verify_local.sh --with-tests    # also run scripts/test_fast.sh
#   scripts/verify_local.sh --with-guardrails-full
#                                         # full scripts/check_guardrails.sh (slow)
#   scripts/verify_local.sh --survey      # print CI parity summary (no gates)
#   scripts/verify_local.sh --with-security-preflight
#                                         # secret/permission scan (+ audit if installed)
#
# Linux system deps (also required by CI quality job):
#   sudo apt-get install -y libfontconfig1-dev
#
# macOS: fontconfig usually comes with Homebrew builds; if desktop2 fails to
# compile, install fontconfig via Homebrew and ensure pkg-config can find it.
#
# Optional: cargo install cargo-machete --locked  (otherwise guardrails skip machete)

set -euo pipefail
cd "$(dirname "$0")/.."

OFFLINE=false
WITH_TESTS=false
FULL_GUARDRAILS=false
SURVEY=false
WITH_SECURITY=false
for arg in "$@"; do
    case "$arg" in
        --offline) OFFLINE=true ;;
        --with-tests) WITH_TESTS=true ;;
        --with-guardrails-full) FULL_GUARDRAILS=true ;;
        --survey) SURVEY=true ;;
        --with-security-preflight) WITH_SECURITY=true ;;
        -h|--help)
            sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown flag: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

if $SURVEY; then
    parity_doc="docs/dev/LOCAL_CI_PARITY.md"
    if [[ -f "$parity_doc" ]]; then
        sed -n '/^## CI job matrix/,/^## Recommended workflows/p' "$parity_doc" | sed '$d'
        echo ""
        echo "Full doc: $parity_doc"
        echo "Run: scripts/verify_local.sh   # pre-push default"
        echo "Run: scripts/verify_local.sh --offline"
    else
        echo "error: missing $parity_doc" >&2
        exit 1
    fi
    exit 0
fi

run_step() {
    local label=$1
    shift
    printf '▸ %s\n' "$label"
    "$@"
}

preflight() {
    if ! command -v python3 >/dev/null 2>&1; then
        echo "error: python3 is required" >&2
        exit 1
    fi
    if $OFFLINE; then
        return 0
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo not found (install rustup, then: rustup update stable)" >&2
        exit 1
    fi
    if ! cargo metadata --format-version 1 >/dev/null 2>&1; then
        cat >&2 <<'EOF'
error: `cargo metadata` failed. Common causes on a fresh clone:
  - Rust toolchain too old for edition 2024 → run `rustup update stable` (this repo pins stable via rust-toolchain.toml)
  - Missing Linux package → `sudo apt-get install -y libfontconfig1-dev`
EOF
        exit 1
    fi
    if [[ "$(uname -s)" == "Linux" ]] && command -v pkg-config >/dev/null 2>&1; then
        if ! pkg-config --exists fontconfig 2>/dev/null; then
            echo "warning: fontconfig.pc not found; desktop2 / all-features builds will fail until you install libfontconfig1-dev (Linux)" >&2
        fi
    fi
}

offline_gates() {
    echo "=== Offline gates (no cargo compile) ==="
    run_step "module declarations resolve" python3 scripts/check_module_files.py
    run_step "oversized-file ratchet" python3 scripts/check_code_size_budget.py
    run_step "oversized-test ratchet" python3 scripts/check_test_size_budget.py
    run_step "panic-prone usage ratchet" python3 scripts/check_panic_budget.py
    run_step "swallowed-error usage ratchet" python3 scripts/check_swallowed_error_budget.py
    run_step "wildcard re-export ratchet" python3 scripts/check_wildcard_reexport_budget.py
    if command -v cargo >/dev/null 2>&1 && cargo metadata --format-version 1 >/dev/null 2>&1; then
        run_step "crate dependency boundaries" python3 scripts/check_dependency_boundaries.py
    else
        echo "⏭  crate dependency boundaries (needs working cargo metadata)"
    fi
    if $WITH_SECURITY; then
        echo ""
        run_step "security preflight (non-strict)" bash scripts/security_preflight.sh
    fi
    echo ""
    echo "=== Release automation (CI release-automation job) ==="
    run_step "Discord release unittest" python3 -m unittest -v scripts/test_post_discord_release.py
    run_step "release script py_compile" \
        python3 -m py_compile scripts/post_discord_release.py scripts/test_post_discord_release.py
}

main_gates() {
    preflight
    if $FULL_GUARDRAILS; then
        run_step "full guardrails" bash scripts/check_guardrails.sh
    else
        run_step "guardrails (--skip-slow)" bash scripts/check_guardrails.sh --skip-slow
    fi
    echo ""
    echo "=== Release automation (CI release-automation job) ==="
    run_step "Discord release unittest" python3 -m unittest -v scripts/test_post_discord_release.py
    run_step "release script py_compile" \
        python3 -m py_compile scripts/post_discord_release.py scripts/test_post_discord_release.py
}

if $OFFLINE; then
    offline_gates
else
    main_gates
    if $WITH_SECURITY; then
        echo ""
        run_step "security preflight (non-strict)" bash scripts/security_preflight.sh
    fi
fi

if $WITH_TESTS; then
    echo ""
    run_step "fast test loop" bash scripts/test_fast.sh
fi

echo ""
echo "✅ Local verification passed."
echo "Next steps: scripts/test_fast.sh (inner loop), scripts/test_ci_suites.py (CI-shaped suites), full guardrails without --skip-slow before large merges."
