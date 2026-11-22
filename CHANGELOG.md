# Changelog

User-visible changes per release. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/). Until 1.0 every release is a beta: it works, but may still change
or break. New changes go under "Unreleased"; a release turns that heading into its version and date.

## [Unreleased]

### Fixed
- Esc closes the installation menu in the sidebar.
- Saving a CurseForge key while Culm is still scanning or checking for updates now rescans once that finishes, so
  CurseForge matches and updates show without pressing Rescan.

### Changed
- Updated the Rune UI framework: when the button that has keyboard focus greys out, focus moves on to the next control
  instead of being lost, and on Windows saving settings no longer fails when another program briefly holds the file.
- Starting Culm while it already runs brings the open window forward instead of opening a second one (Windows, Linux).
- Notifications in the Moonlit and Ledger themes no longer show a coloured stud on their top edge; themes with
  studded frames keep it.

### Fixed
- Installing or updating an addon never replaces a folder that is a developer's git checkout.
- If Culm is closed mid-install (crash, power loss), the addon's old folders are put back at the next scan instead of
  staying hidden, and its source link is kept.
- "Out of date" and the game versions in the detail are right straight after an install or update, not only after
  the next scan.
- One addon folder that can't be read no longer stops My Addons from loading: it is skipped with a warning.
- A zip installed by hand is checked and installed from the same file, so a file replaced in between is never
  installed unchecked.

## [0.2.0] - 2026-11-21

Culm now runs on Windows and macOS too.

### Added
- Windows and macOS: a Windows installer (per user, no admin rights) and a macOS .dmg for Apple Silicon and Intel Macs.
  They are not code-signed yet: the README says how to allow the first start. They check for a new version at start;
  "Update to x.y.z" opens the release page.
- Windows and macOS: Culm finds WoW Forever in its usual install folders (Battle.net's default on any drive,
  `/Applications` on macOS), keeps your addons' sources when you switch from WowUp, and waits with maintenance
  (clear cache, clean backups, back up settings) while the game is running, as on Linux.
- My Addons marks addons made for an older game version "Out of date" (the game loads them only with "Load out of date
  AddOns" on); the tooltip says which versions the addon lists.
- Settings → About → "Copy debug info" copies what helps fix a problem (version, system, game folders, recent errors and
  crashes) for a bug report, and "Report a bug" opens the form. Culm keeps a small log file of errors and crashes;
  your user name and CurseForge key stay out of both.
- Settings → About links a privacy note (which services Culm talks to and what it sends; no telemetry) and lists the
  open-source code and fonts Culm is built from, with their licences. Every download carries that list and Culm's licence.

### Changed
- Addons whose author allows downloads only through CurseForge's own app or website can now be installed and updated:
  Culm opens the download in your browser and installs the file as soon as it lands in your Downloads folder, then
  deletes it. A file already in Downloads is installed right away; one saved elsewhere goes in with "Install from file…"
  in the details. Culm only installs a file that is exactly the expected version. "Update all" skips these.
- Drag the header band to move the window, double-click it to maximize. On Windows and macOS it replaces the system
  title bar, with the window buttons in the band. Linux keeps the system title bar for now.
- Switching an addon between Stable, Beta and Alpha checks only that addon, not all of them.
- Updated the Rune UI framework: a notched mouse wheel scrolls smoothly on Windows and Linux, and on GNOME (Wayland)
  the window's bottom corners are rounded like its title bar.

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

[Unreleased]: https://github.com/gengurke/Culm/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/gengurke/Culm/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/gengurke/Culm/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/gengurke/Culm/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/gengurke/Culm/releases/tag/v0.1.0
