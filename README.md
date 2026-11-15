# Culm

An addon manager for **World of Warcraft: Forever** on Linux, written in Rust on the
[Rune](https://github.com/anomalyco/rune) GUI framework.

**Status:** early but usable. Detects Forever clients in Wine/Proton/Faugus/Lutris/Bottles prefixes, imports
existing addon records, scans and groups `Interface/AddOns`, updates/installs/removes addons from CurseForge,
WowUp Hub, GitHub (WowInterface is implemented but disabled: no game-version info for Forever), and has client maintenance (cache, WTF leftovers, WTF backups). What exactly works and
how it is tested: [docs/FEATURES.md](docs/FEATURES.md).

## Build and run

```sh
sudo dnf install libxkbcommon-devel wayland-devel   # Fedora build deps
cargo build --release
target/release/culm
```

Without root: a gitignored `.cargo/config.toml` can set `LIBRARY_PATH` to a shim dir containing
`libxkbcommon.so -> /usr/lib64/libxkbcommon.so.0` (see `AGENTS.md`). The first build takes a few minutes (Rune is big).

A desktop entry template is in `packaging/dev.culm.app.desktop`: set `Exec` to your binary and copy it to
`~/.local/share/applications/` if you want a launcher; copy `packaging/culm.svg` to
`~/.local/share/icons/hicolor/scalable/apps/culm.svg` for its icon.

## CurseForge API key

CurseForge (search, update checks, fingerprint matching) needs your own free API key:

1. Sign in at <https://console.curseforge.com> and create an API key (section "API keys").
2. In the app: **Settings → CurseForge → API key**, paste it, **Save**. It is stored only on this computer.

Without a key the app still scans, imports existing records, updates GitHub addons and offers updates
the import already knew about; CurseForge addons show "Not checked".

## Where data lives

| What | Where |
|---|---|
| Installations, addon lists per installation | `~/.local/share/dev.culm.app/` (`installations.json`, `addons/<id>.json`) |
| WTF backups | `~/.local/share/dev.culm.app/wtf-backups/<installation id>/` |
| Settings (API key, default channel) | `~/.config/dev.culm.app/` |
| Thumbnail cache | `~/.cache/dev.culm.app/` |

`RUNE_DIRS_ROOT=/some/dir` moves all of them (handy for a test run). Imported files are only read, never written.

## Tests

```sh
cargo test                     # headless, no network, never touches your real WoW folder
cargo clippy --all-targets
```

- `crates/core/tests/*.rs`: feature tests of the core against temp WoW trees and a local fake HTTP server.
- `crates/app/tests/e2e_*.rs`: the real UI driven headlessly (Rune `Runtime`) over the real core and the same fakes.
- `crates/app/tests/ui_shell.rs`: UI-only checks over the mock backend.

Test names carry the feature id from `docs/FEATURES.md` (`f3_2_...`). Contributor notes: `AGENTS.md`.

## License

Copyright (C) 2026 the Culm contributors.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public
License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
version. It is distributed WITHOUT ANY WARRANTY; see [LICENSE](LICENSE) for the full text.

World of Warcraft is a trademark of Blizzard Entertainment. Culm is a community project, not affiliated with or
endorsed by Blizzard.
