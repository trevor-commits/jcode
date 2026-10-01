# Local verification vs GitHub CI

This doc maps `.github/workflows/ci.yml` jobs to local commands. It is the reference behind `scripts/verify_local.sh --survey`.

**Pre-push default:** `scripts/verify_local.sh` (Quality Guardrails with `--skip-slow` + Release Automation Python checks).

**Fully offline (no `cargo` compile):** `scripts/verify_local.sh --offline` (Python ratchets, optional secret/permission scan, release-automation unittest).

## Open draft PR survey (usage-burn reliability)

As of the deeper pass, the only open **draft** PR on `trevor-commits/jcode` for this work should be the successor to [#16](https://github.com/trevor-commits/jcode/pull/16) (`cursor/usage-burn-reliability-deeper-bbe0`). Earlier draft `cursor/usage-burn-verify-local-32e6` is superseded by that branch.

| PR / branch | Status | Scope |
|-------------|--------|--------|
| #16 `cursor/usage-burn-verify-local-32e6` | Superseded | Initial `verify_local.sh`, `rust-toolchain.toml`, CONTRIBUTING links |
| Deeper draft `cursor/usage-burn-reliability-deeper-bbe0` | Active draft | CI parity matrix, `--survey`, offline security scan, doc cross-links |

## CI job matrix

| CI job | Runner | Covered locally | Local command |
|--------|--------|-----------------|---------------|
| **Quality Guardrails** | `ubuntu-latest` | Partial (default verify) | `scripts/check_guardrails.sh` or `scripts/verify_local.sh` |
| **Release Automation** | `ubuntu-latest` | Yes (default verify) | `python3 -m unittest -v scripts/test_post_discord_release.py` + `py_compile` (wrapped in verify) |
| **Format** | `ubuntu-latest` | Via guardrails | `scripts/check_guardrails.sh` (includes `cargo fmt --check`) |
| **Build & Test** | `ubuntu-latest`, `macos-latest` | Partial | `python3 scripts/test_ci_suites.py` (lib-bins, provider-matrix, e2e); `scripts/test_fast.sh` for inner loop |
| **Build & Test** — security preflight (Linux only) | Linux matrix leg | Optional | `scripts/security_preflight.sh` (CI uses `--strict` + installs `cargo-audit`) |
| **Build & Test** — warning budget (Linux only) | Linux matrix leg | Via full guardrails | `scripts/check_warning_budget.sh` (in guardrails, not `--skip-slow` offline path) |
| **windows-build-test** | `windows-latest` | No | Windows machine or wait for CI |
| **powershell-syntax** | `windows-latest` | No (on Linux) | `./scripts/check_powershell_syntax.ps1` on Windows |
| **setup-friction** | `ubuntu-latest` | Manual | `bash scripts/test_install_conversion.sh`, `bash scripts/setup_friction_eval.sh` |
| **windows-cross-check** | `ubuntu-latest` | Manual | See job in `ci.yml` (cross-target `cargo check`) |

### Quality Guardrails step-by-step

| CI step | `verify_local.sh` default | `check_guardrails.sh` | `--skip-slow` |
|---------|---------------------------|----------------------|---------------|
| `check_module_files.py` | Indirect (guardrails) | Yes | Yes |
| `cargo fmt --check` | Indirect | Yes | Yes |
| `cargo check --all-targets --all-features` | Skipped | Yes | No |
| `cargo clippy -D warnings` | Skipped | Yes | No |
| `cargo metadata --locked` | Indirect | Yes | Yes |
| Warning / size / panic / swallowed-error ratchets | Indirect | Yes | Yes |
| `check_dependency_boundaries.py` | Indirect | Yes | Yes |
| `check_wildcard_reexport_budget.py` | Offline + guardrails | Yes | Yes |
| desktop2 frame budget test | Indirect | Yes | Yes |
| `cargo machete` | Skipped | If installed | No |

**Profile note:** CI runs `cargo test -p jcode-desktop2 profile::`. Local guardrails use `--profile selfdev` for the same test module to reuse selfdev artifacts; behavior under test is the same sweep.

**Toolchain note:** CI `build` matrix sets `RUSTUP_TOOLCHAIN: stable` so dependency crates cannot override the channel via nested `rust-toolchain.toml`. Local dev uses repo-root `rust-toolchain.toml` (also `stable` + clippy/rustfmt). Run `rustup update stable` before full guardrails.

**Linux deps:** `sudo apt-get install -y libfontconfig1-dev` (required for desktop2 / all-features compile).

## Recommended workflows

```bash
# 1) Print this matrix summary (no gates)
scripts/verify_local.sh --survey

# 2) Pre-push on a laptop (~minutes, no provider keys)
scripts/verify_local.sh

# 3) No compiler / on a plane
scripts/verify_local.sh --offline

# 4) Before a large merge or when CI Quality job failed
scripts/check_guardrails.sh          # install cargo-machete for parity with CI
rustup update stable

# 5) CI-shaped tests (long; needs resources)
python3 scripts/test_ci_suites.py

# 6) Optional: secret scan + script permissions (CI strict also needs cargo-audit)
scripts/verify_local.sh --with-security-preflight
```

## Explicitly out of scope for `verify_local.sh`

- Provider credentials and `scripts/real_provider_smoke.sh`
- `scripts/memory_regression_gate.sh` (pinned local session)
- Windows installer / lifecycle PowerShell suites
- Refactor shadow builds (`scripts/refactor_phase1_verify.sh`)
- Network install steps unless you opt in (`cargo-audit`, `cargo-machete`)

## Verify steps (agent / maintainer checklist)

Run on Linux with network for `cargo fetch` unless using `--offline` only:

```bash
cd "$(git rev-parse --show-toplevel)"
scripts/verify_local.sh --survey
scripts/verify_local.sh --offline
# optional: sudo apt-get install -y libfontconfig1-dev
rustup update stable
scripts/verify_local.sh
scripts/check_guardrails.sh --skip-slow   # should match verify default gates
```

Do not commit secrets; `security_preflight` scans tracked files for common patterns.
