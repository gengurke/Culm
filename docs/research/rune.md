# Rune: research notes for the WoW addon manager

Researched 2026-11-15 from `https://github.com/anomalyco/rune`, branch `dev`.

**Pinned rev: `b39e1b4d326e323e971b86b8f105d3df53d4642c`** (dev HEAD, 2026-11-21 03:26 -0600, "fix(rune-kit): display math wakes and rebuilds only what a finished render changes (#1317)"; bumped from `8fbc3951`). The root `Cargo.toml` holds the rev; this line only records when the notes were last checked against it.

Main sources in the repo: `README.md`, `AGENTS.md`, `crates/rune/README.md` (the ~3000-line framework guide), `crates/rune-kit/README.md`, `examples/hello`, `examples/todo`, `examples/background`, `CHANGELOG.md`, and the root `Cargo.toml` (`[patch.crates-io]`).

---

## 1. What Rune is

- **A native Rust GUI framework.** It draws everything on the GPU itself (wgpu, plus its own Metal backend on macOS), with its own text stack (Parley/swash, Inter embedded), Taffy flexbox/grid layout, and spring animation (`rune-motion`). There is **no webview, no DOM and no JS**. It is not a TUI. Windowing is winit 0.30 (vendored and patched). It runs on X11 and Wayland on Linux.
- **Platforms:** macOS (Metal), Windows (wgpu DX12/Vulkan), Linux (wgpu Vulkan/GL, X11 and Wayland), iOS (embedded), and Web (WebGPU, experimental).
- **UI model:** like Elm/SwiftUI, immediate-mode-style views over retained state.
  - `fn view(s: &State, cx: &mut Cx<State>) -> El` is a pure function of the app state. It builds a keyed element tree with `row()/col()/el()/text()` and chained style methods.
  - Handlers (`.on_press(|s: &mut State| ..)`) are queued and run between frames with `&mut State`. Then the view runs again, Rune diffs the keyed tree, and animated property tracks are retargeted.
  - Hover and press never re-run the view.
  - Nothing animates on the first frame.
- **One state type per window.** Rune has no component-local state, signals or hooks. Everything lives in your `State` struct, or in the runtime (scroll offsets, focus, springs).
- **Headless-first tooling.** Every app binary in a debug build accepts flags such as `--snap out.png`, `--tree tree.json`, `--script s.json` and `--video`. These render with the same renderer on a virtual clock and need no display. `Runtime` drives the UI in `cargo test`: tap by key, step the clock, read rects and state.
- **Crates in the repo:**

  | Crate | What it is |
  | --- | --- |
  | `rune` | Core |
  | `rune-kit` | Components themed like `@opencode/ui`: Button, Switch, Select, Menu, Table, Tree, Toast, Progress, Spinner, Markdown, the shell (`SplitView`, `Sidebar`, `Toolbar`), and more |
  | `rune-notify` | Notifications (**macOS and Windows only**) |
  | `rune-mac` | macOS helpers |
  | `rune-i18n` | Message catalogs |
  | `rune-fx`, `rune-light` | Effects |
  | `rune-mermaid` | Mermaid diagrams |
  | `themes/opencode` | OpenCode's theme as data |

- **The project's own process is agent-driven.** "We don't take pull requests. Have your agent open an issue." The `dev` branch is fast-forward-only and is never rewritten, so a pinned rev stays fetchable.

## 2. Consuming it before a release

The repo's guidance (README "Add it" and crates/rune/README "Depending on Rune") is:

1. Add Rune as a git dependency pinned to a `dev` revision. Do not vendor it, and do not depend on the branch.
2. Copy **every entry of Rune's root `[patch.crates-io]`** into your own workspace root, at the **same rev**. Cargo does not inherit `[patch]` from a dependency.
   - This is not optional: issue #850 says Rune no longer builds without the winit patch.
   - The five entries are `winit`, `parley_engine`, `wgpu-hal`, `accesskit_windows` and `accesskit_macos`. Cargo finds each one by package name inside the git repo, under `vendor/`.
3. Start from `examples/hello` and check your work headlessly.
4. When you bump the rev, re-read Rune's root `Cargo.toml` at the new rev. Use the same rev everywhere, and add any new patch entries.

There is no scaffold or template command and no CLI binary. You use `cargo new` and copy hello.

### Copy-paste setup

```sh
sudo dnf install libxkbcommon-devel wayland-devel   # build-time link deps (Fedora); see §6
cargo new culm && cd culm
```

`rust-toolchain.toml`, the `rune`/`rune-kit` dependencies, the mirrored `[patch.crates-io]` and the profiles: copy them
from Culm's root `rust-toolchain.toml` and `Cargo.toml`, which are the single source for the pinned rev (the bump
procedure is in AGENTS.md, "Rune rules"). Rune's own choices behind them: the toolchain matches Rune's pin, and
`debug = "line-tables-only"` because full debuginfo made a test build use 18-23 GB RAM. Optional rune features:
`native-file-dialog` for the folder picker, `tooling` for headless flags in release; rune-kit's `markdown` for
addon descriptions/changelogs.

`src/main.rs` is the minimal app, adapted from `examples/hello`:

```rust
use rune::tok::{self, motion, type_};
use rune::{AppSpec, Cx, El, col, el, text};

#[derive(Default)]
pub struct State { on: bool }

pub fn view(s: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    let knob = el().key("knob").size(18.0, 18.0).radius(9.0).fill(t.knob)
        .absolute(if s.on { 18.0 } else { 2.0 }, 2.0);
    let switch = el().key("switch").role("switch").label("Lights").checked(s.on).focusable(true)
        .size(38.0, 22.0).radius(11.0)
        .fill(if s.on { tok::accent(0, true) } else { t.control_off })
        .motion(motion::SNAPPY)
        .on_press(|s: &mut State| s.on = !s.on)
        .child(knob);
    col().w_full().h_full().center().gap(16.0).fill(t.bg)
        .child(text(if s.on { "Lights on" } else { "Lights off" }).font(type_::TITLE).color(t.text))
        .child(switch)
}

fn main() {
    rune::run(AppSpec::<State> {
        scenes: &[("on", |s| s.on = true)],          // named states for --snap --scene on
        ..AppSpec::new("Hello", (320.0, 200.0), State::default, view)
    });
}
```

### Verified on this machine (Fedora 44, rustc 1.99.1, Intel UHD iGPU, Wayland session)

The scratch app lives at `/tmp/claude-1000/rune-research/hello-rune`. It is the hello example plus a rune-kit `Button`, built against the `rune`/`rune-kit` deps, `[patch.crates-io]` and profiles of Culm's root `Cargo.toml`.

**Results:**

| Step | Result |
| --- | --- |
| First `cargo build` (git fetch of about 160 MB, 293 crates) | ~3 min |
| Linking | **Failed:** `rust-lld: error: unable to find library -lxkbcommon`, because `libxkbcommon-devel` is not installed. Fix below. |
| Incremental rebuild | ~1 s |
| `cargo test`: a `Runtime` test that taps the switch and the kit button by key | Passed in 0.06 s |
| `target/debug/hello-rune --snap hello.png [--scene on]` | Correct PNG. GPU init about 230–440 ms on Vulkan (wgpu); no display needed. |
| `--tree tree.json` | JSON accessibility and element tree (keys, roles, names, bounds) |
| `--script e2e/flip.json --tree after.json --at mark:end` | Taps by key worked: the tree showed "Lights on" and "Clicked 2" |
| Window on Wayland, and on X11 (`WAYLAND_DISPLAY=` forces XWayland) | Ran with no errors until `timeout` killed it |
| `cargo build --release` | 2 min 22 s. Binary is 38 MB, **29.5 MB stripped**, much bigger than the README's 10.9 MB macOS figure (Linux links the ICU word dictionaries and zbus/tokio). `ldd` shows only libc/libm/libgcc_s; X11, Wayland and Vulkan are dlopened. |
| `--snap` on the release binary | Refused: "this build has no agent tooling … `features = ["tooling"]`" |

**Link fix:** `sudo dnf install libxkbcommon-devel wayland-devel`.

Scripted e2e file used:

```json
[{"t":0.2,"type":"tap","key":"switch"},{"t":0.5,"type":"tap","key":"count"},
 {"t":0.8,"type":"tap","key":"count"},{"t":1.5,"type":"mark","name":"end"}]
```

The kit button needed a keyed wrapper (`el().key("count").child(Button::..build(&k))`) to be found by key. Kit components key their inner parts (`label`, `control`), so wrap them, or call `.key(..)` on the `El` that `.build(&k)` returns (rune-kit's own docs do `.build(k).key(id)`).

## 3. Structure, state, views, components, IPC

### Shape of an app

- One `State` struct per window.
- `view` functions are free `fn`s, split into helpers that return `El`.
- Handlers mutate state.
- Background work reports back through `poll` / `AppHandle::update`.

There is no UI/backend IPC: everything runs in one Rust process. The "backend" is just your Rust modules called from handlers, or from worker threads that post results back.

### Entry points

| Entry point | What it does |
| --- | --- |
| `rune::run(AppSpec)` | One window. Quits when the window closes. |
| `rune::App::new(spec)` | Builder: `.init(closure)`, `.keep_running()`, `.windowless()`, `.on_close(fn)`, `.on_ready(FnMut(&WindowHandle))`, `.titlebar(..)`, `.window_options(..)`, `.playground(..)`, `.handle()`, `.run()` |
| `rune::run_windows(...)` / `App::window(...)` | Several windows, each with its own state type |

### `AppSpec<S>` fields

These are **fn pointers, not closures**. Build the literal with `..AppSpec::new(title, size, init, view)`.

| Field | Type / purpose |
| --- | --- |
| `title` | `&'static str` |
| `size` | `(f32, f32)` in points |
| `min_size` | `Option<(f32, f32)>`; the default minimum is 420×400 |
| `init` | `fn() -> S` (use `App::init` for a closure) |
| `view` | `fn(&S, &mut Cx<S>) -> El` |
| `scenes` | `&[(name, fn(&mut S))]`: named states for snapshots and the playground |
| `fonts`, `families` | Custom fonts |
| `setup` | `Option<fn(&mut S, Waker)>`: runs once; `Waker = Arc<dyn Fn() + Send + Sync>` |
| `poll` | `Option<fn(&mut S, f64, &mut Tx) -> bool>`: runs before every frame; return true **only if visible state changed** |
| `on_key` | `Option<fn(&mut S, &KeyEvent, &mut Tx) -> bool>` |

### Rules the repo insists on (AGENTS.md and the "Fast apps" page)

- **Modes as explicit state machines.** Model modes and phases (downloads, requests, retries) as one `enum` plus `fn step(&mut State, Event) -> Effects<Effect>`, using `rune::machine::{Attempt, Effects, Seq}`. Do not encode modes with clusters of bools.
- **Timers are events.** Use `tx.wake_at(due)` or `tx.wake_after(dur)`, then handle the timer in `poll`.
- **Start slow work after the first frame.** Start network work and folder scans from `on_ready` or later. Land state that arrives early with `tx.instant()`.
- **Never do file I/O or blocking work on the UI thread during frames.** Debug builds warn when a handler or `poll` blocks for 8 ms or more.
- **Long lists:** use `virtual_rows` and `memo(id, version, || ..)`, and handlers that find an item by id, never by index. Paint hover with `El::hover`, not with state.
- **Keys:** give `.key("…")` to anything you want to test or animate. Keys are one global namespace (rune-kit uses `label`, `value`, `icon`), so prefix them or use paths such as `"addon-123/update"`.

### Element API cheat sheet (`rune::El`)

- **Constructors:** `col()`, `row()`, `el()`, `text(s)`, `rich(s, spans)`, `icon(&Icon, size)`, `text_input(&TextEdit, style)`, `canvas(shape)`.
- **Layout:**
  - Sizes: `.w() .h() .size(w, h) .w_full() .h_full() .min_h(0.0) .max_w()`
  - Flex: `.grow(g)` (flex: g 1 0), `.shrink()`, `.gap()`, `.padding() .px() .py() .mt()`, `.center() .center_y() .justify_between() .flex_wrap()`
  - Position: `.absolute(x, y)`, `.pin(..)`
- **Style:** `.fill(c) .radius(r) .border(w, c) .shadow(y, blur, c) .gradient(a, b) .opacity() .font(TextStyle) .color(c) .truncate() .wrap()`
- **Motion:** `.motion(Spring)` and the `tok::motion::{SNAPPY, DEFAULT, CALM}` presets; `.transition(Transition::..)`.
- **Interaction:** `.on_press(|s| ..)`, `.on_press_tx(|s, tx| ..)`, `.on_double_press`, `.on_secondary_press(|s, press, tx| ..)`, `.on_drag`, `.on_drop_files(|s, files, tx| ..)`, `.hover(color)`, `.cursor(Cursor::Pointer)`, `.focusable(true)`, `.help("tooltip")`.
- **Scrolling:** `.scroll(ScrollSpec { stick_bottom: false })`. It needs a bounded height: `.h_full()` on ancestors, plus `.min_h(0.0)` on the nested flex columns.
- **Accessibility:** `.role("button") .label("…") .checked(b) .disabled(b)`. This is also what tests and `--tree` see.
- **Tokens:** `rune::tok` has `theme(dark) -> Theme { bg, text, text_secondary, card, card_border, control_off, knob, hover, .. }`, `accent(i, dark)`, `space::S1..`, `radius::SM/MD`, and `type_::{TITLE, BODY, LABEL}`. Colours are OKLCH (`rune::oklch(l, c, h)`).
- **`Tx` (handler context):**
  - Timing and windows: `wake_after`, `wake_at`, `clock`, `instant`, `settle`, `motion(spring)`, `set_title`, `close_window`, `quit`, `minimize`, `resize_window`.
  - Focus and scrolling: `focus(key, ring)`, `scroll_into_view(key)`, `scroll_to(..)`.
  - System: `copy(text)`, `open_url(url)`, `open_file / save_file / open_folder(options, callback)`, `popup_menu(menu, anchor, cb)`.
- **`Cx` (view context):** `system_dark()`, `layout_size(key)`, `is_reduced_motion()`, `titlebar_inset()`.

### Components (rune-kit)

- **The pattern:** `let k = rune_kit::Kit::neutral(cx.system_dark());`, then `Button::new("Install").variant(ButtonVariant::Contrast).on_press(|s: &mut State| ..).build(&k)`.
- **Overlays** (Select, Menu, Popover, Tooltip, Toast) take `.build(&k, &mut layer)`. The view root then becomes `rune_kit::root(body, layer)`.
- **Custom theme:** `Kit::from_palette(PaletteSeed { bg, fg, accent })`, or `Theme::from_json`.
- **Useful parts for an addon manager:**
  - `shell::SplitView` + `Sidebar` + `Toolbar`: a sidebar + content layout.
  - `Table`: virtualised, with sortable and resizable columns and selection in `TableState`. It handles 100k rows.
  - `TextInput` / `Combobox`: search.
  - `Progress`, `ProgressCircle`, `Spinner`: downloads.
  - `Toast`/`ToastRegion`, `Badge`, `Menu` / `context_menu`, `SegmentedControl`, `Switch`, `Checkbox`, `Select`.
  - `Markdown`: needs the `markdown` feature.

### Routing / views

There is no router. Keep an `enum Screen { MyAddons, GetAddons, Settings }` in state and `match` on it in `view`. For full-window screens, `rune_kit::shell::Page` matches `SplitView`'s chrome. Use transitions for animated swaps.

## 4. Capabilities

| Need | Rune provides | Use |
| --- | --- | --- |
| Filesystem read/write/delete dirs | No fs API, by design ("keep persistence APIs free of std::fs" applies to Rune internals) | `std::fs` (`create_dir_all`, `remove_dir_all`, `rename`) **on a worker thread**, never in handlers or frames. `rune::dirs::AppDirs::new(brand::APP_ID)` (`"dev.culm.app"`) gives XDG `config()/data()/cache()/state()/logs()` plus home and Downloads. `RUNE_DIRS_ROOT=/tmp/x` relocates all of them for tests. |
| Folder picker (WoW install dir) | `tx.open_folder(FileDialogOptions{..}, \|s, res, tx\| ..)` with rune feature `native-file-dialog` (XDG portal over zbus on Linux) | Without the feature the callback gets an error. Headless/test: `Runtime::file_dialog()` + `resolve_file_dialog(Some(path))`. |
| HTTP (JSON, binary download) | **None.** The changelog says "Rune has no HTTP client"; `ImageCache::url` refuses network URLs natively | Add `ureq` (blocking, simple) + `serde_json`. Run it on `rune::Latest`, `rune::task::spawn` or a thread, and post results back (see below). |
| Zip extraction | None | `zip` crate (pure Rust) on a worker thread. Extract to a temp dir under cache, then `rename` into `Interface/AddOns`. |
| Spawning processes | None | `std::process::Command` (off the UI thread if it waits). |
| Opening URLs | `tx.open_url("https://..")`: only http, https and mailto; runs `xdg-open` on Linux. Tests read `Runtime::take_url_requests()`. | To open a *folder* in the file manager, call `xdg-open <dir>` yourself through `Command`. `open_url` refuses `file:`. |
| Persisting settings | `rune::prefs::Prefs::open("app.id")`: `get::<T>(k)`, `set(k, &v)`. JSON at `$XDG_CONFIG_HOME/<id>/prefs.json`, written atomically (temp file + rename). `Prefs::memory()` for tests; `RUNE_PREFS_DIR` overrides the location. | Small settings only. Store the addon DB as your own serde JSON in `AppDirs::data()`, written on a worker thread. |
| System tray | `rune::Tray::new(TraySpec::new(id, title, TrayIcon::rgba(w, h, px)).menu(vec![TrayItem::action("quit", "Quit")]), \|ev\| ..)`: StatusNotifierItem over D-Bus (default `dbus` feature), Linux and Windows only | GNOME needs an extension to show it. The callback runs on the tray's thread, so forward events with `AppHandle::update`. Combine with `App::keep_running()`. |
| Notifications | `rune-notify`: **macOS and Windows only**; `available()` is false on Linux | On Linux, use the `notify-rust` crate, or skip notifications and use in-app `Toast`. |
| Window management | `App::keep_running/windowless`; `AppHandle::{open, show, hide, focus, close, minimize, quit, update, is_visible}`; `tx.resize_window/center_window/set_title/toggle_maximized`; `WindowOptions { min_size, max_size, resizable, borderless, always_on_top, visible, transparent, .. }`; `App::titlebar(Titlebar::new(h))` for a custom title bar; `.window_drag()` regions | Placement and topmost are no-ops on Wayland. A custom titlebar on GNOME is buggy (#993). |
| Single instance | `rune::single_instance(brand::APP_ID)` first in `main`: Windows and Linux; returns false when another instance runs (it forwards args/links and raises the window) | |
| Images (addon thumbnails) | `ImageCache` (`file`, `rgba`, `load` bytes); native `url` takes only paths or `file:` | Download the bytes yourself, then call `ImageCache::load(bytes)` or write to cache and use `file`. |
| Clipboard | `tx.copy(text)`, `tx.read_clipboard(..)` | |
| Drag-in files | `.on_drop_files(..)`: X11 only on Linux (Wayland is not reported by winit 0.30) | |
| i18n | `rune-i18n` | |

### Background work: the patterns the repo recommends

**1. `Latest<T>`.** Use it for latest-wins requests such as a search box over an addon catalog.

```rust
// state (built in init):
results: rune::Latest<Vec<Addon>>,     // Latest::default().debounce(Duration::from_millis(250))
// setup: s.results.set_waker(waker);
// handler (each keystroke):
s.results.request(query.clone(), |q| api::search(&q));   // runs on a pool worker
// poll:
fn poll(s: &mut State, _t: f64, _tx: &mut Tx) -> bool {
    let Some(hits) = s.results.take() else { return false }; // true only when an answer landed
    s.hits = hits; true
}
```

**2. Channel + waker.** Use it for long jobs that report progress, such as downloads and installs.

```rust
use std::sync::mpsc;
enum Msg { Progress { id: String, done: u64, total: u64 }, Installed { id: String }, Failed { id: String, err: String } }
struct State { tx: mpsc::Sender<Msg>, rx: mpsc::Receiver<Msg>, waker: Option<rune::Waker>, /* .. */ }

fn setup(s: &mut State, waker: rune::Waker) { s.waker = Some(waker); }

fn start_install(s: &mut State, id: String, url: String) {
    let (tx, wake) = (s.tx.clone(), s.waker.clone().unwrap());
    rune::task::spawn("install", move || {
        let r = (|| -> anyhow::Result<()> {
            let bytes = ureq::get(&url).call()?.into_body().read_to_vec()?;  // ureq 3 API
            // zip::ZipArchive::new(Cursor::new(bytes))?.extract(tmp)?; std::fs::rename(..)?;
            Ok(())
        })();
        let _ = tx.send(match r { Ok(()) => Msg::Installed { id }, Err(e) => Msg::Failed { id, err: e.to_string() } });
        wake();                         // runs poll on the UI thread
    }).ok();
}

fn poll(s: &mut State, _: f64, _: &mut rune::Tx) -> bool {
    let mut changed = false;
    while let Ok(m) = s.rx.try_recv() { s.apply(m); changed = true; }
    changed
}
```

**3. `AppHandle::update`.** Grab `let h = app.handle();` before `run()`; it is `Clone + Send + Sync`. Any thread can then call `h.update(|s, tx| ..)`, and the update runs on the UI thread and commits. This is the simplest option when using `rune::App`.

Use `rune::task::spawn(name, FnOnce)` for pooled work, `task::serve(name, rx, each)` for a long-lived service thread, and `Tx::wake_after` for periodic update checks.

## 5. Testing

Testing has three layers, all headless with no display needed. Pixel output does need a GPU adapter; on CI use lavapipe.

### 1. Unit tests with `rune::Runtime`

These are the primary UI tests. There is no renderer and no GPU; the clock is virtual.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime, Key, KeyEvent};

    #[test]
    fn install_button_starts_install() {
        let mut rt = Runtime::new(State::default(), view, (900.0, 600.0), Env { system_dark: true, scale: 2.0 });
        rt.frame(0.0);
        let (x, y) = rt.find("addon-42/install").unwrap();   // key or key path
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        for _ in 0..30 { rt.frame(1.0 / 60.0); }              // advance springs/handlers
        assert!(matches!(rt.state.jobs["42"], Job::Downloading { .. }));
        rt.key(KeyEvent { key: Key::Text("dbm".into()), mods: Default::default() }); // typing into focused field
        rt.update(|s, _tx| s.filter = "x".into());           // mutate state as a handler would
    }
}
```

- **Lookups:** `rt.find(key) -> Option<(x, y)>` (centre); `rt.presented(key)` (drawn rect, mid-animation); `rt.layout(key)`; `rt.elements()` and `rt.element_at(x, y)` return `ElementInfo`, including text; `rt.texts()`.
- **Input:** `rt.pointer_move/down/up`, `rt.wheel(x, y, dy)`, `rt.key(KeyEvent)`, `rt.paste(text)`.
- **System requests:** `rt.take_url_requests()`, `rt.file_dialog()`, `rt.resolve_file_dialog(Some(path))`.
- **Accessibility:** `rt.accessibility_json()`, `rt.accessibility_action(key, action, data)`.
- **Handler checks:** `Runtime::update_if`.
- **Effects:** handlers never really open URLs, dialogs or the clipboard in tests. These become inspectable requests.
- **Background work:** keep I/O behind a trait or channel so tests can inject fakes. Fake `Latest` work runs on real threads; prefer calling `rt.update(|s, _| s.apply(Msg::..))` to simulate arrivals.

### 2. Playground stories

`App::playground(Playground::new().story(Story::new("Addons/Row/Updating", init, view)...))` adds a storybook inside the app (Ctrl+Shift+D). In tests, use `App::new(spec).playground(p).headless_runtime(size, env)`, then `rt.open_playground("Addons/Row/Updating", None)` and `rt.playground_state::<T>()`.

### 3. The binary's headless CLI

Debug builds always accept these flags. Release builds need the rune feature `tooling`.

```sh
B=target/debug/culm
$B --snap out.png [--scene name] [--size 900x600] [--scale 2] [--theme dark|light] [--at 0.4]
$B --tree tree.json                     # element + accessibility tree as JSON (keys, rects, text, roles)
$B --list-scenes
$B --script e2e/install.json --tree t.json --snap s.png     # scripted e2e
$B --script s.json --video run.mp4      # needs ffmpeg
```

Scripts are JSON lists of timestamped events. Events can target an element by `key` instead of x,y:

```json
[{"t":0.2,"type":"tap","key":"addon-42/install"},
 {"t":0.5,"type":"key","name":"Enter"},{"t":0.6,"type":"type","text":"deadly"},
 {"t":1.0,"type":"file-dialog","path":"/fake/World of Warcraft/_retail_"},
 {"t":2.0,"type":"mark","name":"end"}]
```

`rune::tooling::headless_run()` is `Some` inside such a run, so the app can switch to fixture backends: no network, and `RUNE_DIRS_ROOT` for files.

The **motion gate** (`rune::motion_gate`) can fail tests on visual pops and jitter. It is optional.

## 6. Build, run, package (Linux)

### Build-time packages and toolchain

- Build-time packages (Fedora): `libxkbcommon-devel wayland-devel`.
- Runtime: `libxkbcommon-x11`, and a Vulkan driver (`mesa-vulkan-drivers`; lavapipe is `/usr/share/vulkan/icd.d/lvp_icd.json` for GPU-less CI).
- The toolchain must be the one Rune pins (1.99.1 at this rev).

### Commands

```sh
cargo run                         # window (X11 or Wayland)
cargo run -- --snap /tmp/a.png    # headless still
cargo test                        # Runtime tests
cargo build --release             # measured here: 38 MB, 29.5 MB stripped (Linux); add rune feature "tooling" to keep --snap/--tree in release
```

Hot reload exists (`scripts/rune-hot --manifest ../app/Cargo.toml --patch src/view.rs`, which needs `dx` 0.7.10 and the app feature `hot = ["rune/hot"]`). It is geared to the maintainers' setup, and `--local` runs it locally. Treat it as optional.

### Packaging

Rune ships **no packaging tooling for Linux**; `scripts/mac-app` is macOS only. Use the normal Rust options:

- `cargo build --release` + strip, plus a `.desktop` file and icon.
- `cargo-deb` / `cargo-generate-rpm`.
- AppImage (`cargo-appimage` or linuxdeploy).
- Flatpak (needs the portal for the folder picker, which Rune already uses).

Window class / app id: call `rune::set_app_id(brand::APP_ID)` (`"dev.culm.app"`) before `run` (Wayland `app_id`, X11 `WM_CLASS`); name the `.desktop` file to match so the compositor picks up icon and name.

## 7. Gotchas, limitations, unstable areas

- **Pre-release, very fast-moving.** Dozens of commits a day. The CHANGELOG lists behaviour changes per rev and the API grows constantly. Pin a rev and bump it deliberately; on each bump, re-sync `[patch.crates-io]` and the toolchain.
- **`[patch.crates-io]` is mandatory.** Issue #850: it fails to build without the winit patch. Cargo pulls each patched crate from Rune's `vendor/`.
- **Heavy build.** wgpu, winit, parley, ICU, accesskit, zbus (dbus feature; over async-io, no tokio, since Rune b7309162) and resvg. The first build takes minutes and lots of RAM. Keep `debug = "line-tables-only"`.
- **Layout rules are CSS flexbox.**
  - `.grow()` inside an auto-height column collapses to 0.
  - Nested flex columns need `.min_h(0.0)`, or the inner `.scroll` never scrolls.
  - `.virtualize` needs `.growth(..)`.
  - Debug builds warn about some of these.
- **`AppSpec` callbacks are `fn` pointers.** No capturing closures; state goes in `S`. Use `App::init(closure)` or `AppHandle` for injected services.
- **Adding pub fields breaks literals.** Rune never adds pub fields to `AppSpec`, `Env`, `RunOptions`, `ScrollSpec` and similar, but always build them with `..Default`/`..AppSpec::new()`.
- **Linux specifics:**
  - The tray needs a StatusNotifier host (GNOME needs an extension).
  - No notifications from `rune-notify`.
  - `drop_files`, window placement and topmost don't work on Wayland.
  - `App::titlebar` on GNOME gives a grey Adwaita bar (#993).
  - No app-drawn frame or edge resizing yet (#915).
  - Rune #898 (still open) says the wheel jumps the whole notch distance, also on trackpads; Culm's CHANGELOG
    0.2.0 reports a notched mouse wheel scrolling smoothly on Windows and Linux after a Rune update.
  - Wayland wheel scrolling snaps back in a focused field (#992).
  - Only "Noto Color Emoji" is used for emoji (#973).
  - Without the `dbus` feature, dark mode detection falls back to dark.
  - `popup_menu` (native menus) is unsupported on Linux: `on_pick` gets `None`. Use rune-kit `Menu`/`context_menu`, which are drawn in-app.
- **No HTTP client, zip, fs or process APIs.** Bring crates and keep them off the UI thread. Debug builds warn about stalls of 8 ms or more.
- **`open_url` only does http, https and mailto.**
- **Panics.** A panic in a view or handler is not caught in release builds (#1003 asks for opt-in recovery).
- **Web/wasm builds are experimental.** Irrelevant here.
- **`rune-kit` follows `@opencode/ui` naming.** Docs reference Storybook stories; `Kit::neutral` is the plain theme.
- **Docs live in READMEs.** The canonical guide is `crates/rune/README.md`; read it at your pinned rev. There is no docs.rs. Use `cargo doc --open -p rune` locally.
