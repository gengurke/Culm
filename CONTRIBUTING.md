# Contributing to Culm

Bug reports, ideas and pull requests are welcome. Please follow the [Code of Conduct](CODE_OF_CONDUCT.md);
security problems go through [SECURITY.md](SECURITY.md), not public issues.

## Build and test

Install Rust with [rustup](https://rustup.rs) (the version is pinned in `rust-toolchain.toml`). On Linux also the
Wayland and xkbcommon headers (Fedora: `sudo dnf install libxkbcommon-devel wayland-devel`, Debian/Ubuntu:
`sudo apt install libxkbcommon-dev libwayland-dev`). Windows and macOS need nothing extra.

```sh
cargo run                             # open the window
cargo test                            # all tests, headless, no network
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings
target/debug/culm --snap out.png      # headless screenshot, for UI changes
```

## Rules

[AGENTS.md](AGENTS.md) holds the project rules for people and coding agents alike: layout, UX, code and test rules.
In short:

- Every feature in [docs/FEATURES.md](docs/FEATURES.md) has an id (`F3.1`); its tests carry it in their name
  (`f3_1_...`). Update FEATURES.md and the "Unreleased" part of [CHANGELOG.md](CHANGELOG.md) with the change.
- One topic per pull request, on a branch `<type>/<issue>-<what>` (e.g. `fix/81-sort-scroll-jump`); the body starts
  with `Fixes #<n>`. CI must be green.
- New or changed themes follow [docs/THEMES.md](docs/THEMES.md): a theme is one data file, views never pick colours.
- Tests never use the network or your real WoW folder.
- No proprietary assets: no game art, fonts or other files Culm may not redistribute.
