# Quantus

A desktop calculator with exact arithmetic, units, equations, integrals and curve sketching.
Rust engine, Tauri 2 shell, React + MathLive frontend.

## Branches

Work on `dev`. `master` only gets a merge at release time, see [RELEASING.md](RELEASING.md).
Nothing runs automatically on `dev`; the *Check* workflow is started by hand.

## Layout

- `crates/quantus-core`: the engine, no UI. `eval/`, `electro/`, `linear`, `numeric`, `quantity`,
  `settings`, `session`, `text`, `api`. Tests are integration tests in `tests/`.
- `desktop/`: the Tauri app. Rust shell in `src-tauri/`, frontend in `src/` (`i18n/en.ts` and `de.ts`).
- `docs/`: Docusaurus site; contributor pages in `docs/docs/contributing/`.
- `design/`: specifications. `plans/plans.md`: open work. `test/`: the generated test set.

The electronics mode is implemented but hidden in the UI (see `plans/plans.md`).

## Commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cd desktop && npm run typecheck
cd desktop && npm run dev        # app with hot reload
cargo run --release -p quantus-core --example run_cases   # the generated test set
```

Details: `docs/docs/contributing/building.md` and `testing.md`.

## Conventions

- Add user-visible changes to `## [Unreleased]` in `CHANGELOG.md`.
- UI strings go into both `desktop/src/i18n/en.ts` and `de.ts`.
- After changing a dependency, regenerate `THIRD-PARTY-LICENSES.md` (`npm run licenses` in `desktop/`).
