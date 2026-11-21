# Changelog

User-visible changes per release. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/). Until 1.0 every release is a beta: it works, but may still change
or break. New changes go under "Unreleased"; a release turns that heading into its version and date.

## [Unreleased]

### Added
- Culm for Windows and macOS: a Windows installer (per user, no admin rights) and a macOS .dmg for
  Apple Silicon and Intel Macs. They are not code-signed yet: the README says how to allow the first start.
  They check for a new version when they start; "Update to x.y.z" opens the release page.
- Settings → About links a privacy note: which services Culm talks to and what it sends (no telemetry).
- Settings → About → Licences lists the open-source code and fonts Culm is built from, with their licences.
  Every download (AppImage, Windows installer, macOS app) also carries that list and Culm's own licence.
- Windows and macOS: switching from WowUp keeps your addons' sources, as on Linux (Culm reads WowUp's settings there too).
- Windows and macOS: maintenance tools (clear cache, clean backups, back up settings) wait while the game is running, as on Linux.
- Windows and macOS: Culm finds WoW Forever in its usual install folders (Battle.net's default on any drive, `/Applications` on macOS).

### Changed
- An update whose author only allows downloads through CurseForge's own app or website no longer fails: its Update button starts the download in your browser, and Culm installs the file as soon as it is in your Downloads folder, then deletes it (and any duplicate copy). If the file is already in Downloads, it is installed right away without opening the browser. Saved it somewhere else? "Install from file…" in the details. Culm only installs a file that is exactly the new version. "Update all" skips these.
- New addons with the same restriction (e.g. Platynator) are no longer greyed out in Get Addons: Install fetches them the same way, through your browser and Downloads folder.
- Drag the header band to move the window, double-click it to maximize. On Windows and macOS it replaces the system title bar: the window buttons sit in the band (on Windows in the theme's colours). Linux keeps the system title bar for now.
- Switching an addon between Stable, Beta and Alpha checks only that addon, not all of them.
- Updated the Rune UI framework: a notched mouse wheel scrolls smoothly on Windows and Linux, and on GNOME (Wayland) the window's bottom corners are rounded like its title bar.

## [0.1.2] - 2026-11-17

### Fixed
- After an update in the app, the restart no longer leaves a helper process of the old version running until Culm closes.
- My Addons says "No addons installed yet" for an empty installation instead of "No addons match “”".

## [0.1.1] - 2026-11-17

### Changed
- Nothing new to see: this release makes sure updating from 0.1.0 works, in the app and with Gear Lever.

## [0.1.0] - 2026-11-17

First beta, Linux only (AppImage).

### Added
- Finds World of Warcraft: Forever clients in Wine, Proton, Faugus, Lutris and Bottles prefixes; more can be added by hand.
- My Addons: scans `Interface/AddOns`, groups folders into addons, matches them to CurseForge, WowUp Hub and GitHub,
  and updates or removes them.
- Get Addons: one search over CurseForge (with your own API key) and WowUp Hub; install from a GitHub link.
- Imports what WowUp already installed.
- Maintenance: clears the game cache and leftover backup files, and backs up the settings folder.
- Five themes: Moonlit, Hearthwood, Emberglow, Ledger and Grimoire.
- Updates itself: the AppImage checks for a new version at start and installs it with one click and a restart.
- The AppImage carries update information, so Gear Lever, AppImageUpdate and AppImageLauncher can update it too.

[Unreleased]: https://github.com/gengurke/Culm/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/gengurke/Culm/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/gengurke/Culm/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/gengurke/Culm/releases/tag/v0.1.0
