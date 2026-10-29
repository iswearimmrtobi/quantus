# Preparing the repository for going public (2026-10-24)

What was checked, changed and found while preparing `github.com/iswearimmrtobi/quantus` to become
public. Open work from this is in [plans.md](https://github.com/iswearimmrtobi/quantus/blob/master/plans/plans.md); how to run the tests is in
[testing.md](testing.md).

## Status

| | |
|---|---|
| License | MIT, added on GitHub (`9797065`) |
| Secrets in history | None found |
| Build output in git | None; `target/`, `node_modules/`, `.idea/` are ignored |
| Dependency licenses | All MIT or Apache-2.0, compatible with MIT |
| Personal email in history | Replaced with the GitHub no-reply address (local only, see below) |
| Tests | `cargo test --workspace` passes, `npm run typecheck` passes |
| **Still to do** | Force push, then make the repository public on GitHub |

## Licensing

The concern: with MIT, anyone may use `quantus-core` in a closed product and sell their own
interface for it, as long as they keep the copyright notice. The alternatives that were considered:

- **GPL-3.0**: does not forbid selling, but any distributed app built on the core has to be GPL and
  open source too, which in practice rules out closed paid forks.
- **MPL-2.0 or LGPL-3.0** on `quantus-core` only: changes to the core must stay open, a closed
  interface around it is allowed.
- **No license**: all rights reserved; people can view and fork on GitHub but not legally use the
  code.

MIT was chosen.

## Changes

| Commit | Change |
|---|---|
| `d95dace` | Translated the remaining German metadata (`package.json` description, `tauri.conf.json` short and long description, a `.gitignore` comment). The German search keywords in the `.desktop` file stay, so German users still find the app. |
| `a7bfe22` | App identifier `de.quantus.desktop` → `dev.tobifrosch.quantus` (the old one implied owning `quantus.de`). This moves the app's config directory, which is fine before a release. |
| `13d3df9` | Fixed the failing Cauchy-integral golden case, see below. |
| `0f25e88` | `docs/testing.md`: every test and how to run it. |
| `8a29219` | `docs/plans.md`: open work. |
| `81d6097` | Removed finished docs: `phase0-findings.md` (old Kotlin library spikes), `algebra-simplification-plan.md` (implemented), `migration-rust-tauri.md` (migration finished; its open manual checks moved to `plans.md`). Links in the README and the spec updated. |

The test cases in `test/` and `crates/quantus-core/tests/golden/` stay in the repository: they show
the engine is tested and let others reproduce runs. The largest file is 0.5 MB, far below GitHub's
100 MB limit, and nothing private was found in them.

## Email in the git history

13 commits had a personal email address as author email. All commits on `master` were rewritten
with `git filter-branch` to use `103638753+iswearimmrtobi@users.noreply.github.com`; names were not
changed. The local git config already used the no-reply address.

- Every commit hash changed. Other clones of the repository have to be cloned again.
- A backup of the history before the rewrite is at `/home/tobi/RustroverProjects/quantus-backup.git`.
  It still contains the old email: do not push it anywhere, and delete it once the push has worked.
- The rewrite is only local so far. Publish it with:

  ```sh
  git push --force-with-lease origin master
  ```

- **Do not pull before this push.** `git pull`, or RustRover's *Update Project* after a rejected
  push, rebases the local commits onto the old history on GitHub and undoes the rewrite. This
  happened once on 2026-10-24, and the rewrite had to be run again. A rejected push is expected
  here; `--force-with-lease` is the answer to it, not a pull.
- After the push, check that `git log --format='%ae%n%ce' | sort -u` shows only the no-reply
  address and `noreply@github.com` (GitHub's committer address for commits made in its web UI).

## The failing Cauchy-integral test

`cargo test --workspace` failed on one case in `tests/golden/evaluate.json`:
`\int_{-\infty}^{\infty} \frac{1}{1 + x^2}\,dx` was expected to start with
`3{,}14159265358979323846264338327`, but the engine returned `3{,}14159265358979323846264338328`.

The engine was right. Numerical results are capped at 30 significant digits (`MAX_NUMERIC_DIGITS`
in `eval/evaluator.rs`) and rounded, and π to 30 digits is `…338328` (π = …3383279502…). The
expectation was π truncated instead of rounded. The case failed in the commit that added it, so it
had never passed. The expectation was corrected.

Side effect: `golden.rs` stops at the first failing case, so the 74 cases after this one had never
been checked. They all pass.

## Why the tests seemed slow

While fixing the test, runs took minutes and one appeared to hang for 26 minutes. Measured on its
own, the whole golden suite (254 cases) takes about 4–6 s, and the whole workspace about 10 s plus
build time. Most of the apparent slowness came from how the investigation was run: several test runs
at once sharing the `target/` directory with each other and with RustRover, and waiting on runs
instead of measuring single cases.

The real finding: three improper integrals of exponentials take 0.8–1.2 s each, while rational
integrands take under 0.1 s:

| Case | Time |
|---|---:|
| `\int_1^{\infty} \frac{1}{x^2}\,dx` | 0.05 s |
| `\int_{-\infty}^{\infty} \frac{1}{1 + x^2}\,dx` | 0.07 s |
| `\int_{\infty}^{1} \frac{1}{x^2}\,dx` | 0.02 s |
| `\int_0^{\infty} e^{-2x}\,dx` | 0.84 s |
| `\int_{-\infty}^{\infty} e^{-x^2}\,dx` | 0.83 s |
| `\int_{-\infty}^{0} e^{x}\,dx` | 1.18 s |

A trace of the integrator showed about 1,100 samples per integral, with single samples close to the
infinite end taking up to 130 ms. There, `x = t/(1-t)` is an exact fraction with about 100-digit
numerator and denominator. A single `e^{-x}` is fast even for huge x, so the cost lies in building
and evaluating those fractions. The rule is that no test may take more than a second;
`tests/golden.rs` breaks it. The fix is item 1 in [plans.md](https://github.com/iswearimmrtobi/quantus/blob/master/plans/plans.md), and making `golden.rs`
report every failing case is item 2.
