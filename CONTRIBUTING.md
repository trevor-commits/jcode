# Contributing to jcode

Thanks for contributing.

## Local verification

Before you push, run the same lightweight gates CI enforces on every PR:

```bash
# Pre-push default (~minutes, no provider keys; needs current stable Rust)
scripts/verify_local.sh

# Fully offline (Python ratchets + release-automation unittest only)
scripts/verify_local.sh --offline

# Inner-loop Rust tests (minimal feature profile)
scripts/test_fast.sh

# CI-shaped test suites with per-suite timeouts
python3 scripts/test_ci_suites.py
```

**Toolchain:** the workspace uses Rust edition 2024. `rust-toolchain.toml` pins the `stable` channel (with clippy and rustfmt). Run `rustup update stable` if `cargo` complains about `edition2024`.

**Linux deps:** `scripts/check_guardrails.sh` and full `cargo check --all-features` need fontconfig development headers, same as CI:

```bash
sudo apt-get install -y libfontconfig1-dev
```

**Full guardrails** (format, clippy, machete, desktop2 frame budget, all ratchets): `scripts/check_guardrails.sh`. Use `--skip-slow` for a faster pass without `cargo check` / clippy / machete. See `AGENTS.md` for maintainer workflow notes.

**CI parity matrix:** `docs/dev/LOCAL_CI_PARITY.md` lists every `ci.yml` job and the local equivalent. Quick summary: `scripts/verify_local.sh --survey`. Agent offline playbook (commands only): `scripts/verify_local.sh --checklist`.

**macOS-only / network-heavy checks** are not part of `verify_local.sh` (for example Windows smoke workflows, real-provider smoke scripts, memory regression gates that need a pinned local session). Run those only when you are working in those areas.

## Issues vs pull requests

If the problem is easy for me to reproduce, please prefer opening a GitHub issue. A clear issue with reproduction steps, expected behavior, actual behavior, logs, screenshots, or traces is usually the fastest path to a fix.

Pull requests are more useful when the problem depends on an environment I may not have, such as macOS-specific behavior, Windows-specific behavior, unusual shells, terminal emulators, filesystems, GPU/display setups, provider accounts, or other local configuration. In those cases, a PR can be a useful reference because it captures the behavior in the environment where the problem actually occurs.

## Pull request policy

Pull requests are welcome and encouraged.

That said, most PRs should be treated as proposals or references, not as changes that are likely to be merged directly. This project is developed with heavy use of code generation, and generated code can be deceptively plausible: it may fix the visible problem while introducing subtle correctness, lifecycle, architecture, or maintenance issues.

Because of that, I will often use PRs to understand the bug, feature request, test case, design direction, or proposed implementation, then write my own version of the change. The submitted code may still be extremely valuable as a reference, reproduction, or proof of concept, even if the final committed code is different.

This is not a judgment that maintainer-generated code is inherently better than contributor-generated code. It is a practical ownership rule: if I am going to maintain the resulting code, I need to understand its assumptions, tradeoffs, and failure modes.

The best PRs therefore include:

- a clear description of the problem being solved
- a minimal reproduction or failing test when possible
- notes about edge cases and tradeoffs
- focused changes that are easy to review independently
- any relevant logs, screenshots, traces, or benchmarks

Large, generated, or highly invasive PRs may be closed even when the underlying idea is good. In those cases, the issue or PR may still be used as a reference for a maintainer-authored change.

Handwritten by author: My clanker slop may or may not be better than your clanker slop. I know how to work with my clanker slop though.
