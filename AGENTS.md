# AGENTS.md

**Culm**, an addon manager for **World of Warcraft: Forever**, written in Rust on the
[Rune](https://github.com/anomalyco/rune) GUI framework. The name lives only in `crates/app/src/brand.rs` and the
`[[bin]]` name in `crates/app/Cargo.toml`.

Culm is cross-platform and should look and behave the same everywhere. Targets:
- **Linux with KDE Plasma** (Wayland and X11): the main target; everything must work here.
- **Windows** and **macOS**: fully supported, same features.
- **Linux with GNOME**: supported. Where GNOME lacks something (e.g. the tray needs an extension), the feature
  may be missing or limited there, but nothing errors or breaks. Small GNOME tweaks are fine; workarounds that
  complicate the code or hurt other platforms are not.
- Other desktops and distros (Mint/Cinnamon, XFCE, …): welcome when they work, no extra code unless asked.

Write platform-neutral code: Rune and std APIs (`std::path`, `rune::dirs`, no hard-coded `/` or `~`), nothing
visual from the system (fonts are bundled, see Themes). Per-OS code only where no neutral way exists: small,
in one place behind one function, with a sensible fallback elsewhere. Issues and PRs that touch platform
behaviour say what happens on each target.

Where files go: every file Culm writes goes in the `rune::dirs` folder that fits its kind; never invent paths.
`config` holds settings the user chose. `data` holds records Culm keeps and can't rebuild (installed addons,
backups, downloaded class icons). `cache` holds anything rebuildable; deleting it must be safe. `state` and `logs`
hold machine-local runtime state and logs. These follow each OS's convention (XDG on Linux, `~/Library/…` on
macOS, AppData on Windows). On Windows `data` roams with the profile (`%APPDATA%`) while `cache` and `state` stay
local (`%LOCALAPPDATA%`), so keep large or machine-specific files out of `data`.
One exception: installs work next to `AddOns` in the WoW `Interface` folder (`.addons-staging-*`, `-backup-*`,
`-trash-*`, `-kept-*`, so swaps are renames on one filesystem) with `.addons-lock` beside them, so every process
touching that folder, whatever its data root, sees the same lock.

## Layout

```
crates/core   addon-core: UI-free, blocking Rust. WoW installs, TOC, fingerprints, providers,
              install/update/remove, maintenance, persistence. No Rune dependency.
crates/app    addon-app: Rune UI (state, views, theme, background workers). Thin over core.
crates/testkit addon-testkit (dev-dependency only): FakeServer (local HTTP), provider JSON builders + fixtures, make_zip.
docs/FEATURES.md   what the software does (keep it current!)
docs/research/     background research (Rune guide)
```

## Commands

```sh
cargo build                     # first build ~1-3 min (Rune is big), then seconds
cargo test                      # all tests, headless (no display/GPU needed for Runtime tests)
cargo run                       # open the window
target/debug/culm --snap out.png [--size 1100x720] [--scene settings@hearthwood]   # headless screenshot
target/debug/culm --tree tree.json                  # element tree (keys, text, rects)
scripts/tree-diff.sh target/trees-before            # every scene's tree; on the refactor branch add
scripts/tree-diff.sh target/trees-after target/trees-before   # the before folder: prints what changed
```

Development happens on Linux; keep the build free of Linux-only steps for macOS and Windows.
Build dependencies (Fedora): `sudo dnf install libxkbcommon-devel wayland-devel`.

## Workflow

- Every change goes on its own branch and into `master` through a pull request; nothing is committed to `master`
  directly (GitHub enforces this).
- Branch names: `<type>/<issue>-<what>`, lowercase kebab-case, type one of `feat`, `fix`, `docs`, `chore`
  (e.g. `fix/81-sort-scroll-jump`; no issue → `docs/workflow`). Rename tool-made names (`claude/…`, `worktree-…`)
  before pushing.
- One topic per PR. Title says what changes for the user; the body starts with `Fixes #<n>` and briefly says what,
  why and how it was tested. CI must be green; the user merges.
- Several agents may work on this machine at once: edit only in your own git worktree, never in the main checkout.
  No `git stash` (shared by all worktrees), no `git add -A`/`commit -a`, don't touch other branches, and never
  merge or push `master` unless the user asks.
- After the merge, clean up: remove your worktree, the local and remote branch, and any leftovers of the change
  (scratch builds, temp files).

## Releases

SemVer, 0.x while in beta (`brand::STAGE` shows "Beta" beside the version). Every user-visible change adds a line under
"Unreleased" in `CHANGELOG.md`. To release: turn "Unreleased" into `## [x.y.z] - <date>` (and the links at the bottom),
bump `version` in the root `Cargo.toml`, add `<release version="x.y.z" date="..."/>` to
`packaging/dev.culm.app.metainfo.xml` on a `chore/release-x.y.z` branch, merge its PR, then tag the merged `master`:
`git tag vx.y.z && git push origin vx.y.z`.
`.github/workflows/release.yml` checks all three, runs the tests, builds the AppImage (`packaging/appimage.sh`) and
publishes the release. Never mark a release "pre-release": the AppImage's zsync update info only follows `latest`.

## Rune rules (from Rune's own guidance — see docs/research/rune.md)

- Use Rune as it is meant to be used and follow its conventions, even while it is in development: Culm grows
  with it. On a limit or bug, first check whether Rune already covers it or we use it wrong (its docs,
  newer `dev` commits, its issues); if it is Rune's, propose an upstream issue.
- No workarounds around Rune unasked: propose one to the user and build it only once agreed. Then keep it
  small and in one place, marked `// shortcut: <Rune issue>, remove once fixed`, so it goes when Rune fixes it.
- Rune is pinned to one `dev` commit in the root `Cargo.toml`. On a bump: same rev everywhere, re-copy
  Rune's root `[patch.crates-io]`, match `rust-toolchain.toml`, read Rune's CHANGELOG.
- One `State` per window; `view(&State, &mut Cx<State>) -> El` is pure. Handlers mutate state.
- Never block the UI thread: all fs/network work runs on workers (`rune::task::spawn`) and comes back
  through a channel + waker (`poll`) or `AppHandle::update`.
- Modes/phases are explicit enums (no bool clusters). Timers are events (`tx.wake_after`).
- Give `.key("...")` to everything tests touch. Keys are global: use paths like `addon/<id>/update`.
- Layout is flexbox: scroll areas need bounded height (`.h_full()` up the chain, `.min_h(0.0)` on nested cols).

## Rune pitfalls (learned here)

- `virtual_rows`' key fn replaces the row's own `.key()`: return the full path key (`addon/<id>`).
- Virtual rows below the fold aren't mounted, so `rt.find` returns None: select via state or keep test lists short.
- `tx.focus` doesn't reach a kit `TextInput` nested in a keyed row: set `edit.focused = true` in state.
- `.border()` and `.border_sides()` replace each other; for a hairline plus a side accent use an inset
  `box_shadow` with spread 1.
- `AppSpec.fonts` replaces Rune's embedded Inter, so Culm bundles its own (`theme::FONTS`, Rune's `embedded-fonts`
  feature off). `Runtime::new` uses Rune's default text system: tests go through `addon_app::runtime` (`with_text`).
- No const hex colour constructor: `LazyLock` + `rune::color::hex`.
- `Runtime::new` has no setup/poll: set `rt.poll` (see `addon_app::runtime`); `setup` doesn't run in tests, so
  anything created there (the thumbnail `ImageCache`) must be set by the test.
- `resolve_file_dialog` returns a `#[must_use]` Result.
- A clear border insets a box's children and inset shadows, not its fill or gradient: an inset plaque (`nav_active`) paints
  its face with a wide inset shadow (`spread` past half its height).
- Tooltips (`.help(..)`) take the theme through `cx.help_look(ui::help_look())` in `view`; tests read them as
  `ElementInfo::help` and `rt.help_tooltip()`.
- A text's `max_w` + `truncate` cuts what it draws, but a row hugging its content still sizes to the full text.
- `ElementInfo::rect` is the animated rect (a row entering can be 1 px tall); use `target` for layout assertions.
- `.pixels(img)` paints when the image decodes without re-running the view: put it over a fallback instead of
  branching on `is_ready()`.
- Headless binary runs (`--snap`, `--script`) use `MockBackend`; `ADDONS_BACKEND=core` forces the real one
  (with `RUNE_DIRS_ROOT=$(mktemp -d)` and the `ADDONS_*_URL` endpoints pointed at a closed port it stays offline).

## Themes (`crates/app/src/theme/`)

- A theme is data: `Pal` (colours), `Type` (text styles per role, `t().ty.body`) and `Skin` (named `Surface`s: fill,
  gradient, procedural texture, border/bevel, radius, shadows, hover, corner ornaments). Views and `ui.rs` ask the
  current theme (`theme::t()`, `pal()`) and paint boxes with `.skin(&t().skin.panel)`: no raw hex, no theme checks.
  Sizes and spacing (`theme::PAD`, `ROW_H`, …) are global, so layout is the same in every theme.
- `.skin()` adds the texture and ornaments as pinned children: call it before `.child(..)`.
- The current theme is a thread-local set by `theme::begin` at the top of `view` (tests run windows on parallel threads).
  Textures (`theme/texture.rs`, value-noise fBm, no image files) are generated once per process on background threads (`theme::prewarm` from `setup`; synchronously in tests and headless runs) and registered per thread through `State::images`;
  without a cache surfaces keep their base colour.
- Adding a theme: one file `theme/<name>.rs` with `pub static THEME: LazyLock<Theme>` (start from `moonlit.rs`) and one
  `Variant => module` line in the `themes!` list in `theme/mod.rs`. The picker, prefs id, scenes (`<scene>@<id>`) and
  `f7_3_every_theme_renders_every_screen` pick it up. Fonts: every family a theme names
  (`Family::Named`, one bundled name, no fallback list) is bundled in the binary with its licence (OFL/Apache) in
  `assets/fonts/<family>/` and listed in `theme::FONTS`, so text looks identical on every OS
  (`f7_3_every_theme_font_is_bundled`); never rely on system fonts, and never bundle proprietary game fonts or assets.
- A light-paged theme (Grimoire) sets `Theme::frame` to a second, dark theme: views build the sidebar and the header band
  inside `theme::framed(|| ..)`, where `t()`/`pal()` return the frame (`Theme::frame_look`). `Skin::glyphs` swaps stock glyphs per theme
  (`theme::glyph`, applied in `ui::button`/`icon_button`/`glyph_tile` and the nav). Theme-only glyphs (emblems, corner
  studs, swaps) live in `theme/glyphs.rs`, the band decoration's drawing in `theme/deco.rs`, never in `icons.rs` or `view/`. `Surface::press` sets the pressed wash.
  `skin.menu` is the floating popup (dropdowns, installation menu): give it a full border and a drop shadow.
- Keep textures on big surfaces (window, sidebar, band, detail); cards use gradients and bevels (GPU cost per textured box).
  `f7_3_every_theme_keeps_text_readable` checks palette contrast. `Skin::medal` holds the class medallions' ring tones.

## UX rules

- Simple beats configurable: pick a default that fits most people. Add a setting only when people with real
  reasons need different behaviour, never "in case someone wants it".
- Make risky actions safe instead of scary: back up first and offer undo/restore afterwards, rather than
  confirmation dialogs.
- Plain words in the UI (no WDB, WTF, SavedVariables in labels); details go into tooltips.
- One obvious way to do a thing; every control explains itself (tooltip) and greys out with a reason when it
  can't run.
- Push back: when a request or issue adds options, steps or jargon that make Culm harder to understand, say so
  and propose the simpler version before building it.

## Sources

- Open by design: support as many addon sources as we can (CurseForge, WowUp Hub, GitHub, WowInterface, Wago, …),
  merged into one list. People see addons, not providers: one card per addon, its sources as a detail, one Install.
- A new source must not break anything: it sits behind the `Provider` trait, existing records and links keep
  working, a source that is down or not set up only loses its own results (no errors elsewhere), and sources
  that need no key work out of the box.
- Use every source only the official, allowed way and follow its terms: respect authors' choices (no download
  URL → no install), store nothing a source forbids storing, send keys only to their own hosts, no proxies or
  scraping around limits. When unsure, read the terms and record the outcome in an issue.
- Setup stays light: everything optional and skippable, settings reachable later; a missing key only shows
  where it matters.

## Data-safety rules (learned here)

- Worker results about an installation carry its id (`Msg::Scanned { inst, .. }`); `State::apply` drops stale ones.
- `CoreBackend`: store edits go through `edit` (one lock); installs/updates/removes share the `disk` RwLock, a scan
  takes it alone. Remove/update only touch `store::owned_folders` (stored record minus folders others list).
- Without a key, TOC-id matches (WowInterface) must never take over a stored link of another provider (`merge_rescan`).
- Smoke-check the real install read-only: `RUNE_DIRS_ROOT=$(mktemp -d) timeout 8 target/debug/culm`. With
  `RUNE_DIRS_ROOT` set, Culm skips `rune::single_instance`, so it starts beside a running Culm instead of only bringing
  that window forward.

## Code rules

- Smallest change that fully solves the task. Reuse before writing; std before a crate; no speculative
  abstractions, options or "for later" code. Deleting beats adding. Refactor freely when it makes the
  whole simpler.
- Modular but not over-split: core modules have one job each and talk through `addon_core::model` types
  and the `providers::Provider` trait.
- Never cut validation at trust boundaries (zip paths!, API data), error handling that prevents data loss
  (install backup/rollback), or safety checks (no maintenance while WoW runs).
- Shortcuts get a comment: `// shortcut: <limit>, <when to upgrade>`.
- Planned work, bugs and shortcuts worth upgrading live in GitHub issues (`gh issue list -R gengurke/culm`), not in TODO
  files. Open an issue for work found in passing instead of doing it unasked; a commit that settles one says `Fixes #<n>`.

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
