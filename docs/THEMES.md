# Themes

How Culm's looks work and how to make a new one. The code lives in `crates/app/src/theme/`. Every field named here has
a doc comment in `theme/mod.rs`; this page is the map.

## What a theme is

A theme is **one file**, `theme/<name>.rs`, holding `pub static THEME: LazyLock<Theme>`. A `Theme` is plain data:

- `Pal`: the colours views read.
- `Type`: one text style per role.
- `Skin`: named `Surface`s that components paint, plus the emblem, glyph swaps, band decoration and medallion tones.
- `id`, `name`, `blurb`, and an optional `frame` (see [Frame themes](#frame-themes)).

What a theme file may do:

- Build colours (`h("#…")`, `ink(a)`, `oklcha`).
- Keep its own tokens as local `let`s: sidebar wood, band sky, field wells and the like.
- Use the shadow helpers from `theme/mod.rs` and fill slots from `Skin::base`.

What it must not do:

- Contain logic or reach into views.
- Change sizes or spacing. `SIDEBAR_W`, `BAND_H`, `PAD`, `ROW_H` and `CONTROL_H` are global, so layout is identical in
  every theme.

The other side of the contract: views and `ui.rs` never decide a look. They ask `theme::t()` / `pal()`, paint boxes
with `.skin(&t().skin.panel)` and colour text from `Pal`. `f7_3_views_leave_looks_to_the_theme`
(`crates/app/tests/e2e_themes.rs`) fails on raw hex, `oklch`, `ThemeId::`, `.frame` or `Color::BLACK/WHITE` in
`view/` and `ui.rs`. Its short allow-list says why each exception exists (the class medallion paints from the class
colour, not the theme).

## Adding a theme, step by step

1. Copy `theme/moonlit.rs` (the smallest) to `theme/<name>.rs`. Set `id` (stable: it is stored in prefs), `name` and a
   one-line `blurb`.
2. Add one `Variant => module` line to the `themes!` list in `theme/mod.rs`. The picker, the prefs id, the scenes
   (`<scene>@<id>`) and the every-theme tests pick it up from there.
3. Write the `Pal` (see the table below). Run `cargo test f7_3_every_theme_keeps_text_readable` early: it checks
   contrast.
4. Write the `Type`: `Type { heading: .., ..BASE_TYPE }`, changing only what differs. A new font family needs
   [bundling](#fonts).
5. Write the `Skin` as `Skin { panel: .., primary: .., ..Skin::base(&p, radius) }`. `base` fills the plain slots from
   the palette. The slots it leaves blank, which every theme sets, are `sidebar`, `band`, `panel`, `detail`, `menu`,
   `row_selected`, `primary`, `secondary`, `input`, `tile`, `segments`, `toggle_on` and `toast`, and `medal`
   (`Medal::default()` is transparent).
6. Look at it: `target/debug/culm --snap out.png --scene swatches@<id>` shows every colour, text role, surface,
   status mark and glyph swap on one screen; colours, text roles and surfaces carry their slot names. Then check every screen with
   `cargo run -- --list-scenes` and `--scene <scene>@<id>`. `scripts/tree-diff.sh` dumps every scene's element tree.
7. Run `cargo test` (`f7_3_every_theme_renders_every_screen`, `f7_3_every_theme_font_is_bundled`). Then add the theme
   to F7.3 in `docs/FEATURES.md` and to `CHANGELOG.md`.

## Pal

Every colour must meet its contrast rule on both `bg` and `panel` (WCAG ratio; translucent colours are composited over
`bg`). `f7_3_every_theme_keeps_text_readable` checks it for the page palette and for the frame's.

| colour | where it shows | rule |
|---|---|---|
| `bg` | the window ground; the kit's dropdowns and sliders | base of the test |
| `panel` | card ground, for the test; make it `skin.panel`'s fill | base of the test |
| `text` | body text, the window caption, tooltips | 4.5:1 |
| `heading` | page titles, section headings | 4.5:1 |
| `text_dim` | descriptions, inactive tabs, quiet badges | 4.5:1 |
| `text_muted` | hints, counts, timestamps, key caps, icon buttons | 3:1 |
| `border` | edge of quiet badges, stock key-cap outline | — |
| `border_strong` | scroll thumbs, a tooltip edge when `skin.menu` has no border | — |
| `divider` | hairlines between rows and sections | — |
| `hover` | hover wash on rows, chips, cards | — |
| `chip` | quiet badge ground, tooltip key caps | — |
| `track` | progress bar track | — |
| `accent` | links, active controls, progress, the chosen card's ring, the scroll thumb under the pointer | 3:1 |
| `accent_hi` | the lit side of accent (band embers) | — |
| `accent_focus` | keyboard focus rings, the focused field's and the open dropdown's ring | — |
| `success`, `warning`, `danger`, `busy` | status `Tone`s, read through `ui::Mark` | `ink` 3:1 |

A `Tone` has three colours:

- `ink`: text and glyphs.
- `soft`: the wash behind badges and notices, usually `ink` at 0.10–0.17.
- `line`: their 1 pt edge, usually `ink` at 0.4–0.5.

`busy` is usually the accent.

## Type

`BASE_TYPE` is Moonlit's ladder. Each role has one use; call sites tweak size or weight only rarely
(`StyleExt::size_of` / `weight_of`).

| role | where |
|---|---|
| `display` | the first-run welcome's title |
| `hero` | the page title in the header band |
| `title` | the addon name in the detail panel |
| `heading` | section headings, card and toast titles |
| `body` | running text, field text, menu entries |
| `strong` | the addon name in a My Addons row |
| `label` | buttons, segments, dropdowns (Emberglow sets small caps here) |
| `note` | the band's meta line, notes under sections, empty states |
| `small` | row details, sidebar sub-lines, tooltips |
| `number` | counts and status numbers (keep `'tnum'`) |
| `caption` | key caps, initials tiles |
| `eyebrow` | spaced capitals over a group ("CLASS", sidebar groups) |
| `mono` | versions, paths, ids (keep `Family::Mono`) |

## Skin

A `Surface` has these fields:

- `fill`: the ground, the fallback while a texture loads, and what the contrast test reads.
- `ink`: text and glyph colour, where the component reads it.
- `vertical`: a top-to-bottom gradient.
- `overlay`: a second linear or radial gradient.
- `texture`: a texture and its opacity.
- `border`, `radius` and `shadows`.
- `hover` and `press`: washes. A transparent `press` means 1.6× `hover`.
- `corners`: a glyph pinned into each corner.

`.skin(&surface)` paints all of it. Call it before `.child(..)`, because textures and corners are pinned children.

| slot | painted by | fields that matter |
|---|---|---|
| `window` | the window ground behind every page | `fill`, `ink` (default text), `texture` |
| `sidebar` | the left column | all; often textured |
| `band` | the page header band | all; often textured |
| `band_deco` | what draws over the band (`BandDeco::None/Stars/Embers`) | — |
| `toolbar` | the strip under the band (filter, search) | `fill`, `border` |
| `panel` | addon, picker, maintenance and settings cards; notices (`ui::notice`) take its `radius` | all but `texture` |
| `detail` | the addon detail panel at the right | all; may be textured |
| `menu` | floating popups (installation menu, dropdown list); tooltips take its fill, border colour, radius (≤ 6) and outer shadows | full border + drop shadow |
| `list_header` | My Addons' column header | `fill`, `border` |
| `row` | a My Addons row | `border` (separator), `hover` |
| `row_selected` | the row whose detail is open | all |
| `nav` / `nav_active` | sidebar rows; the label in `ink`, the glyph in `accent` (active) or `text_muted` | all; `nav_active` is often an inset plaque |
| `primary` | the main action (Install, Update all) | all; `ink` 4.5:1 on `fill` |
| `secondary` | other buttons, the dropdown button | all; `ink` 4.5:1 on `fill` |
| `ghost` | quiet buttons: no frame, a hover wash | `ink`, `border`, `hover` |
| `danger` | buttons that delete (Remove, Clean) | all |
| `icon_button` | glyph-only buttons (close the detail, show the key, toast close) | `ink`, `radius`, `hover`, `press` |
| `input` | text fields; its radius rounds the kit's dropdowns and sliders | all |
| `toggle` | an unselected class chip or source row; its shape also draws the sidebar's update count | all but `ink` (unselected text is `text_dim`) |
| `toggle_on` | the selected class chip, source row or segment | all; `ink` colours its text |
| `segments` | the track of a segmented control; unselected segments are bare (`hover` wash, `text_muted` text) | all |
| `tile` | square wells: initials tiles, glyph tiles, toast item slots | all; `ink` colours the glyph |
| `toast` | framed alerts; a `corners` glyph also crowns the top edge as a gem | all; no `texture` (see Textures) |
| `kbd` | key hint boxes (`/`) | `ink`, `border`, `radius` |
| `medal` | the class medallions' ring: `ring`, `ring_on` (gilded, selected), `outline` | — |
| `emblem` | the emblem on the first-run welcome (`ui::emblem`); the sidebar's brand mark stays the stock one | — |
| `glyphs` | `(stock, own)` glyph swaps, looked up by `theme::glyph` | — |

`Skin::base(&p, radius)` fills `window`, `toolbar`, `list_header`, `row`, `nav`, `nav_active`, `ghost`, `danger`,
`icon_button`, `toggle` and `kbd` from the palette, with `radius` on the small controls.

## Surface recipes

The shadow helpers live in `theme/mod.rs`. Shadows paint in order, and inset ones sit inside the border.

- **Hairline**: `border: Some(Border::all(1.0, p.border))`. This is Moonlit's whole card.
- **Bevel**: `Border::bevel(w, lit, shade)`. With uniform widths, a `top` colour that differs from `bottom` draws a
  lit top and a shaded bottom. Add `inset(1.0, light)` for a highlight under the top edge and `inset(-1.0, dark)` for a
  shade above the bottom.
- **Rings**: `inner_ring(c)` draws a 1 pt line just inside the edge, and `outer_ring(c)` one just outside. Stack them for
  carved frames, e.g. Emberglow's stone: bevel, a dark inset at spread 3, a gold inset at spread 4, an outer black ring
  and a `drop`.
- **Drop**: `drop(y, blur, alpha)` is a black shadow below the box.
- **Inset plaque** (Hearthwood's `nav_active`): `Border::margin([t, r, b, l])` is a clear border that
  leaves a gap round the plaque. Paint the face with `face(colour)`, an inset shadow wider than half the height,
  because a `fill` would run under the clear border.
- **Gradients**: use `vertical` for top-to-bottom. `Overlay::Linear` takes `angle` in radians (CSS: 0 is bottom to top,
  `PI` is top to bottom). `Overlay::Radial` takes `center` as fractions of the box and `radius` in points, which suits a
  glow from one edge (Emberglow's amber from below).
- **Corners**: `Corners { glyph: &glyphs::STUD, size, inset, color }` pins an ornament into each corner.
- **Textures** (`Tex::Wood`, `Parchment`, `Stone`, `Leather`, `Moss`, generated in `theme/texture.rs`) go on big
  surfaces only: window, sidebar, band, detail. Every textured box costs GPU time, so cards, toasts and controls use
  gradients and bevels instead. A texture sits inside a uniform border, at the given opacity over fill and gradient.
  The theme's texture list is derived from its skin. A new texture is code: a `Tex` variant plus a generator.
- **Press**: `press` sets the pressed wash. Bright buttons look better darkening (`h("#…").alpha(0.25)`) than with the
  default stronger hover.

States are drawn by components, over the skin, never by replacing its border:

- The focused field and the open dropdown get a 1 pt `accent_focus` ring inside the border.
- Chosen cards get a 2 pt `accent` ring.
- Keyboard focus gets a ring just outside the box.
- Status boxes are `ui::notice`: the tone's `soft` and `line`, with `panel`'s radius.

A theme therefore only has to make its surfaces look right at rest, hovered and pressed.

## Frame themes

A theme with light pages (Grimoire) needs a dark frame round them. It sets `frame: Some(&FRAME)`, where `FRAME` is a
second `Theme` in the same file. Most of its skin is copied from the page theme (`..ps.clone()`); only what the frame
shows changes.

Views build the sidebar and the header band inside `theme::framed(|| ..)`. In there, `t()` and `pal()` return the frame
(`Theme::frame_look`), so the sidebar and band read the frame's palette and skin. The page skin's `sidebar` and `band` stay
as fallbacks. The contrast test checks both palettes. Use a frame only when one palette can't serve both the pages and
the chrome.

## Glyphs and decorations

- Glyphs only themes use (emblems, corner studs, swaps) go in `theme/glyphs.rs`, never in `icons.rs`, which holds
  the stock set. `Skin::glyphs` swaps a stock glyph for the theme's own. `ui::button`, `icon_button`, `glyph_tile` and
  the nav apply the swap through `theme::glyph`.
- `BandDeco` picks the band decoration. A new one is a variant plus its drawing in `theme/deco.rs`.
- Draw new glyphs yourself or take them from a permissively licensed set (Lucide, ISC), and note the source in a comment.

## Fonts

Every family a theme names (`Family::Named("Cinzel")`, one name, no fallback list) is bundled in the binary, so text
looks the same on every OS. To add one:

1. Put the font file and its `OFL.txt` in `crates/app/assets/fonts/<family>/`. Only OFL fonts so far: the licence
   script collects `*/OFL.txt`, so a font under another licence needs the script extended first.
2. Add an `include_bytes!` line to `theme::FONTS`.
3. Regenerate `THIRD-PARTY-LICENSES.md` with `packaging/third-party-licences.sh`.

`f7_3_every_theme_font_is_bundled` fails on a family that isn't bundled. Never rely on system fonts, and never bundle
proprietary game fonts, art or other assets Culm may not redistribute.

## Shared on purpose

These look the same in every theme by design. Don't look for a knob:

- Sizes and spacing (`SIDEBAR_W`, `BAND_H`, `PAD`, `ROW_H`, `CONTROL_H`) and the scroll bar widths (3 pt, 6 pt
  under the pointer; colours from `border_strong` and `accent`).
- The focus ring (1 pt `accent_focus`, 2 pt out), the selected ring (2 pt `accent`), the disabled opacity
  (`ui::DISABLED`, and 0.6 for a locked card), and the round status badge (`ui::badge`).
- The primary button's label weight (600).
- The class medallion's disc: it is shaded from the class colour, not the theme; only its ring comes from
  `Skin::medal`.
- The brand wordmark and its emblem glow.
- The toast layout (glyph slot, title, close button); its frame comes from `skin.toast`.
