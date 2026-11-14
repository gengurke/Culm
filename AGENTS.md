# AGENTS.md

Addon manager for **World of Warcraft: Forever**, written in Rust on the
[Rune](https://github.com/anomalyco/rune) GUI framework. Product name is not final: it lives only in
`crates/app/src/brand.rs` and the `[[bin]]` name in `crates/app/Cargo.toml`.

## Layout

```
crates/core   addon-core: UI-free, blocking Rust. WoW installs, TOC, fingerprints, providers,
              install/update/remove, maintenance, persistence. No Rune dependency.
crates/app    addon-app: Rune UI (state, views, theme, background workers). Thin over core.
docs/FEATURES.md   what the software does (keep it current!)
docs/research/     background research (Rune guide)
```

## Commands

```sh
cargo build                     # first build ~1-3 min (Rune is big), then seconds
cargo test                      # all tests, headless (no display/GPU needed for Runtime tests)
cargo run                       # open the window
target/debug/culm --snap out.png [--size 1100x720]   # headless screenshot
target/debug/culm --tree tree.json                  # element tree (keys, text, rects)
```

Build needs `libxkbcommon-devel wayland-devel` (Fedora). Without root, a gitignored
`.cargo/config.toml` sets `LIBRARY_PATH` to a shim dir containing `libxkbcommon.so -> /usr/lib64/libxkbcommon.so.0`.

## Rune rules (from Rune's own guidance — see docs/research/rune.md)

- Rune is pinned to one `dev` commit in the root `Cargo.toml`. On a bump: same rev everywhere, re-copy
  Rune's root `[patch.crates-io]`, match `rust-toolchain.toml`, read Rune's CHANGELOG.
- One `State` per window; `view(&State, &mut Cx<State>) -> El` is pure. Handlers mutate state.
- Never block the UI thread: all fs/network work runs on workers (`rune::task::spawn`) and comes back
  through a channel + waker (`poll`) or `AppHandle::update`.
- Modes/phases are explicit enums (no bool clusters). Timers are events (`tx.wake_after`).
- Give `.key("...")` to everything tests touch. Keys are global: use paths like `addon/<id>/update`.
- Layout is flexbox: scroll areas need bounded height (`.h_full()` up the chain, `.min_h(0.0)` on nested cols).

## Code rules

- Smallest change that fully solves the task. Reuse before writing; std before a crate; no speculative
  abstractions, options or "for later" code. Deleting beats adding. Refactor freely when it makes the
  whole simpler.
- Modular but not over-split: core modules have one job each and talk through `addon_core::model` types
  and the `providers::Provider` trait.
- Never cut validation at trust boundaries (zip paths!, API data), error handling that prevents data loss
  (install backup/rollback), or safety checks (no maintenance while WoW runs).
- Shortcuts get a comment: `// shortcut: <limit>, <when to upgrade>`.

## Tests

- **Feature tests are the contract.** Each feature in `docs/FEATURES.md` has an id (`F3.1`) and is
  covered by end-to-end-ish tests named after it (e.g. `f3_1_update_replaces_folders`):
  - `crates/core/tests/*.rs`: real temp WoW folders + a local fake HTTP server, through the public core API.
  - `crates/app/tests/*.rs`: drive the real UI headlessly with `rune::Runtime` (tap by key, wait for
    workers, assert on visible text/state) against the same fixtures.
- Unit tests inside modules are disposable details: change or delete them when refactoring.
- **Never write to the user's real WoW folder or another app's config in tests.** Reading them for manual checks
  is fine (your own `World of Warcraft/_forever_` folder).
- No real network in tests. CurseForge needs an API key (user adds their own in Settings);
  never use or extract another app's key.

## Docs upkeep

When behaviour changes or a feature is added, update `docs/FEATURES.md` in the same commit
(1-2 sentences per feature, nested where it belongs, with test names).
